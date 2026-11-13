use axum::{
    Router,
    extract::State,
    http::{HeaderValue, header},
    middleware,
    routing::{get, post},
};
use tower_http::{
    compression::{
        CompressionLayer,
        predicate::{DefaultPredicate, NotForContentType, Predicate},
    },
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

use crate::{
    assets, auth, cierre, cobranza, consumos, dashboard, db, error::AppError, estudiantes, pagos,
    productos, state::AppState,
};

pub fn router(state: AppState) -> Router {
    // Any logged-in user (also `puede_editar = 0`): register consumption, as in Go.
    let con_sesion = Router::new()
        .route("/registro", get(consumos::handler::inicio))
        .route("/registro/{sector}", get(consumos::handler::sector))
        .route("/resumen/{sector}", get(consumos::handler::resumen))
        .route(
            "/consumos/{id_estudiante}",
            get(consumos::handler::editor).post(consumos::handler::guardar),
        )
        .route("/productos", get(productos::handler::listar));

    // Everything else needs `puede_editar = 1`.
    let con_edicion = Router::new()
        .route("/", get(dashboard::handler::index))
        // Payments
        .route(
            "/pagos/{id_estudiante}",
            get(pagos::handler::gestion).post(pagos::handler::registrar),
        )
        .route(
            "/pagos/{id_estudiante}/{id_pago}/eliminar",
            post(pagos::handler::eliminar),
        )
        .route(
            "/comprobante/{id_estudiante}",
            get(pagos::handler::comprobante),
        )
        // Collections and day close
        .route("/cobranza", get(cobranza::handler::index))
        .route("/cierre", get(cierre::handler::index))
        // Configuration
        .route(
            "/estudiantes",
            get(estudiantes::handler::listar).post(estudiantes::handler::crear),
        )
        .route("/estudiantes/{id}", post(estudiantes::handler::actualizar))
        .route(
            "/estudiantes/{id}/estado",
            post(estudiantes::handler::cambiar_estado),
        )
        .route("/productos", post(productos::handler::crear))
        .route("/productos/{id}", post(productos::handler::actualizar))
        .route(
            "/productos/{id}/estado",
            post(productos::handler::cambiar_estado),
        )
        .route_layer(middleware::from_fn(auth::middleware::requiere_edicion));

    Router::new()
        .merge(
            con_sesion
                .merge(con_edicion)
                .route_layer(middleware::from_fn_with_state(
                    state.clone(),
                    auth::middleware::requiere_sesion,
                )),
        )
        .route(
            "/login",
            get(auth::handler::mostrar).post(auth::handler::ingresar),
        )
        .route("/logout", get(auth::handler::salir))
        .route("/health", get(health))
        .fallback(|| async { AppError::NotFound })
        // Every dynamic route: CSRF cookie and POST token check.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::csrf::proteger,
        ))
        // Dynamic pages must never come from the browser cache: going back
        // after a POST would show stale data and invite a duplicate submit.
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        // Added after the layer above, so static assets stay cacheable.
        .route("/public/{*ruta}", get(assets::servir))
        // HTML and CSS shrink ~10x with gzip; fonts and images are already compressed.
        .layer(
            CompressionLayer::new()
                .gzip(true)
                .compress_when(DefaultPredicate::new().and(NotForContentType::const_new("font/"))),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Result<&'static str, AppError> {
    db::ping(&state.db).await?;
    Ok("ok")
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
        response::Response,
    };
    use tower::ServiceExt;

    use super::router;
    use crate::{
        auth::{
            csrf,
            sesion::{self, Sesion},
        },
        state::AppState,
        test_support::{CLAVE, estado, valor},
    };

    async fn enviar(app: &Router, request: Request<Body>) -> Response {
        app.clone().oneshot(request).await.unwrap()
    }

    const CSRF: &str = "token-csrf-de-prueba";

    /// Cookies of a logged-in browser (empty string = anonymous).
    fn cookies(estado: &AppState, puede_editar: bool) -> String {
        let sesion = Sesion {
            id_usuario: if puede_editar { 1 } else { 2 },
            puede_editar,
        };
        let token = estado
            .auth
            .llave
            .firmar(sesion, chrono::Utc::now().timestamp());
        format!("{}={token}; {}={CSRF}", sesion::COOKIE, csrf::COOKIE)
    }

    fn editor(estado: &AppState) -> String {
        cookies(estado, true)
    }

    fn get(cookies: &str, uri: &str) -> Request<Body> {
        Request::get(uri)
            .header(header::COOKIE, cookies)
            .body(Body::empty())
            .unwrap()
    }

    /// Form POST carrying the CSRF header the way `app.js` sends it.
    fn post(cookies: &str, uri: &str, cuerpo: String) -> Request<Body> {
        Request::post(uri)
            .header(header::COOKIE, cookies)
            .header(csrf::CABECERA, CSRF)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(cuerpo))
            .unwrap()
    }

    async fn texto(respuesta: Response) -> String {
        let bytes = to_bytes(respuesta.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn cabecera(respuesta: &Response, nombre: header::HeaderName) -> &str {
        respuesta.headers()[nombre].to_str().unwrap()
    }

    fn extraer<'a>(html: &'a str, antes: &str) -> &'a str {
        let inicio = html.find(antes).expect(antes) + antes.len();
        let fin = inicio + html[inicio..].find('"').unwrap();
        &html[inicio..fin]
    }

    #[tokio::test]
    async fn reenviar_el_mismo_formulario_de_pago_registra_un_solo_pago() {
        let estado = estado().await;
        let s = editor(&estado);
        let app = router(estado.clone());
        let html = texto(enviar(&app, get(&s, "/pagos/1?fecha=2026-10-26&grado=3")).await).await;
        let token = extraer(&html, r#"name="token" value=""#).to_owned();
        let cuerpo = format!("monto=5.00&fecha_pago=2026-10-26&grado=3&token={token}");

        let primero = enviar(&app, post(&s, "/pagos/1", cuerpo.clone())).await;
        assert_eq!(primero.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            cabecera(&primero, header::LOCATION),
            "/pagos/1?fecha=2026-10-26&grado=3&ok=pago"
        );

        let segundo = enviar(&app, post(&s, "/pagos/1", cuerpo)).await;
        assert_eq!(
            cabecera(&segundo, header::LOCATION),
            "/pagos/1?fecha=2026-10-26&grado=3&ok=pago-duplicado"
        );
        assert_eq!(valor::<i64>(&estado, "SELECT COUNT(*) FROM pagos").await, 1);
        assert_eq!(valor::<f64>(&estado, "SELECT monto FROM pagos").await, 5.0);
    }

    #[tokio::test]
    async fn un_pago_sin_token_se_rechaza_sin_escribir() {
        let estado = estado().await;
        let s = editor(&estado);
        let app = router(estado.clone());
        let respuesta = enviar(
            &app,
            post(&s, "/pagos/1", "monto=5&fecha_pago=2026-10-26".to_owned()),
        )
        .await;
        assert!(respuesta.status().is_client_error());
        assert_eq!(valor::<i64>(&estado, "SELECT COUNT(*) FROM pagos").await, 0);
    }

    #[tokio::test]
    async fn guardar_un_consumo_vuelve_a_la_lista_del_sector() {
        let estado = estado().await;
        let s = editor(&estado);
        let app = router(estado.clone());
        let respuesta = enviar(
            &app,
            post(
                &s,
                "/consumos/1",
                "fecha=2026-10-27&sector=menor&cantidad_1=2&cantidad_2=0".to_owned(),
            ),
        )
        .await;
        assert_eq!(respuesta.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            cabecera(&respuesta, header::LOCATION),
            "/registro/menor?fecha=2026-10-27&ok=consumo"
        );
        assert_eq!(
            valor::<f64>(&estado, "SELECT SUM(total_linea) FROM consumos").await,
            3.0
        );
    }

    #[tokio::test]
    async fn las_paginas_no_se_cachean_y_los_estaticos_versionados_si() {
        let estado = estado().await;
        let s = editor(&estado);
        let app = router(estado);
        let pagina = enviar(&app, get(&s, "/?grado=3&fecha=2026-10-27")).await;
        assert_eq!(cabecera(&pagina, header::CACHE_CONTROL), "no-store");

        let html = texto(pagina).await;
        let css = format!(
            "/public/css/app.css?v={}",
            extraer(&html, "/public/css/app.css?v=")
        );
        let estatico = enviar(&app, get(&s, &css)).await;
        assert_eq!(estatico.status(), StatusCode::OK);
        assert_eq!(
            cabecera(&estatico, header::CACHE_CONTROL),
            "public, max-age=31536000, immutable"
        );

        let etag = cabecera(&estatico, header::ETAG).to_owned();
        let revalidado = enviar(
            &app,
            Request::get(&css)
                .header(header::IF_NONE_MATCH, etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(revalidado.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn cobranza_y_cierre_muestran_saldos_y_ventas() {
        let estado = estado().await;
        estado
            .db
            .call(|c| {
                c.execute_batch(
                    "INSERT INTO consumos (id_estudiante, id_producto, cantidad, precio_unitario_venta, fecha_consumo)
                       VALUES (1, 2, 2, 5.0, '2026-10-27');
                     INSERT INTO pagos (id_estudiante, monto, fecha_pago) VALUES (1, 4.0, '2026-10-27');",
                )
            })
            .await
            .unwrap();
        let s = editor(&estado);
        let app = router(estado);

        let cobranza = texto(enviar(&app, get(&s, "/cobranza")).await).await;
        assert!(cobranza.contains("Alva, Ana"));
        assert!(cobranza.contains("S/ 6.00"), "10.00 consumed - 4.00 paid");
        assert!(cobranza.contains("tiene un saldo pendiente de S/ 6.00"));
        assert!(
            !cobranza.contains("Bravo, Luis"),
            "students without debt are not listed"
        );

        let cierre = texto(enviar(&app, get(&s, "/cierre?fecha=2026-10-27")).await).await;
        assert!(cierre.contains("S/ 10.00"), "sold");
        assert!(cierre.contains("S/ 4.00"), "collected");
    }

    #[tokio::test]
    async fn el_comprobante_abierto_desde_cobranza_vuelve_a_cobranza() {
        let estado = estado().await;
        let s = editor(&estado);
        let app = router(estado);
        let html = texto(
            enviar(
                &app,
                get(&s, "/comprobante/1?fecha=2026-10-26&grado=3&desde=cobranza"),
            )
            .await,
        )
        .await;
        assert!(html.contains(r#"href="/cobranza""#));
    }

    #[tokio::test]
    async fn rutas_desconocidas_responden_404() {
        let estado = estado().await;
        let s = editor(&estado);
        let app = router(estado);
        assert_eq!(
            enviar(&app, get(&s, "/no-existe")).await.status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            enviar(&app, get(&s, "/public/no-existe.css"))
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }

    // ---- Login, sessions, CSRF and permissions ----

    fn set_cookies(respuesta: &Response) -> Vec<String> {
        respuesta
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|v| v.to_str().unwrap().to_owned())
            .collect()
    }

    /// Anonymous browser that already loaded the login page (has the CSRF cookie).
    fn anonimo() -> String {
        format!("{}={CSRF}", csrf::COOKIE)
    }

    fn login(usuario: &str, clave: &str) -> Request<Body> {
        post(
            &anonimo(),
            "/login",
            format!("usuario={usuario}&password={clave}"),
        )
    }

    #[tokio::test]
    async fn sin_sesion_se_va_al_login_y_el_login_entrega_la_cookie_csrf() {
        let app = router(estado().await);
        for ruta in ["/", "/registro", "/cobranza", "/productos"] {
            let respuesta = enviar(&app, get("", ruta)).await;
            assert_eq!(respuesta.status(), StatusCode::SEE_OTHER, "{ruta}");
            assert_eq!(cabecera(&respuesta, header::LOCATION), "/login", "{ruta}");
        }

        let pagina = enviar(&app, get("", "/login")).await;
        assert_eq!(pagina.status(), StatusCode::OK);
        let cookies = set_cookies(&pagina);
        assert!(
            cookies
                .iter()
                .any(|c| c.starts_with("kiosco_csrf=") && !c.contains("HttpOnly")),
            "{cookies:?}"
        );
        let html = texto(pagina).await;
        for texto_go in [
            "Iniciar Sesión",
            "Identifícate para continuar",
            "Usuario",
            "Clave",
            r#"name="csrf_token""#,
        ] {
            assert!(html.contains(texto_go), "{texto_go}");
        }

        let salud = enviar(&app, get("", "/health")).await;
        assert_eq!(salud.status(), StatusCode::OK, "health stays public");
    }

    #[tokio::test]
    async fn credenciales_correctas_abren_sesion_de_24_horas() {
        let app = router(estado().await);
        let respuesta = enviar(&app, login("  jenny ", CLAVE)).await;
        assert_eq!(respuesta.status(), StatusCode::SEE_OTHER);
        assert_eq!(cabecera(&respuesta, header::LOCATION), "/");
        let cookies = set_cookies(&respuesta);
        let sesion = cookies
            .iter()
            .find(|c| c.starts_with("kiosco_session="))
            .expect("session cookie");
        for atributo in ["HttpOnly", "SameSite=Lax", "Max-Age=86400", "Path=/"] {
            assert!(sesion.contains(atributo), "{sesion}");
        }
        assert!(!sesion.contains("Secure"), "Secure only when configured");

        // The issued cookie opens the app.
        let valor = sesion.split(';').next().unwrap();
        let inicio = enviar(&app, get(&format!("{valor}; {}", anonimo()), "/")).await;
        assert_eq!(inicio.status(), StatusCode::OK);
        // And the login page sends a logged-in user home.
        let otra_vez = enviar(&app, get(valor, "/login")).await;
        assert_eq!(cabecera(&otra_vez, header::LOCATION), "/");
    }

    #[tokio::test]
    async fn usuario_inexistente_y_clave_incorrecta_dan_el_mismo_mensaje() {
        let app = router(estado().await);
        for (usuario, clave) in [("jenny", "otra"), ("nadie", CLAVE)] {
            let respuesta = enviar(&app, login(usuario, clave)).await;
            assert_eq!(respuesta.status(), StatusCode::UNAUTHORIZED, "{usuario}");
            assert!(
                !set_cookies(&respuesta)
                    .iter()
                    .any(|c| c.starts_with("kiosco_session=")),
                "no session on failure"
            );
            let html = texto(respuesta).await;
            assert!(
                html.contains("Usuario o contraseña incorrectos"),
                "{usuario}"
            );
        }
    }

    #[tokio::test]
    async fn cinco_fallos_bloquean_el_login_aunque_la_clave_sea_correcta() {
        let app = router(estado().await);
        for _ in 0..5 {
            let fallo = enviar(&app, login("jenny", "mal")).await;
            assert_eq!(fallo.status(), StatusCode::UNAUTHORIZED);
        }
        let bloqueado = enviar(&app, login("jenny", CLAVE)).await;
        assert_eq!(bloqueado.status(), StatusCode::TOO_MANY_REQUESTS);
        let html = texto(bloqueado).await;
        assert!(html.contains("Demasiados intentos"));
        assert!(html.contains(
            "Has realizado demasiados intentos de login. Intenta de nuevo en 15 minutos."
        ));
    }

    #[tokio::test]
    async fn un_login_sin_token_csrf_se_rechaza_con_el_mensaje_de_go() {
        let app = router(estado().await);
        let sin_token = Request::post("/login")
            .header(header::COOKIE, anonimo())
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(format!("usuario=jenny&password={CLAVE}")))
            .unwrap();
        let respuesta = enviar(&app, sin_token).await;
        assert_eq!(cabecera(&respuesta, header::LOCATION), "/login?error=csrf");
        assert!(
            !set_cookies(&respuesta)
                .iter()
                .any(|c| c.starts_with("kiosco_session="))
        );

        let html = texto(enviar(&app, get(&anonimo(), "/login?error=csrf")).await).await;
        assert!(html.contains("Token de seguridad expirado"));
        assert!(
            html.contains(
                "El token de seguridad ha expirado. Recarga la página e intenta de nuevo."
            )
        );
    }

    #[tokio::test]
    async fn el_token_csrf_tambien_vale_como_campo_del_formulario() {
        let app = router(estado().await);
        let campo = Request::post("/login")
            .header(header::COOKIE, anonimo())
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(format!(
                "csrf_token={CSRF}&usuario=jenny&password={CLAVE}"
            )))
            .unwrap();
        let respuesta = enviar(&app, campo).await;
        assert_eq!(cabecera(&respuesta, header::LOCATION), "/");
    }

    #[tokio::test]
    async fn un_post_con_sesion_pero_sin_csrf_no_escribe() {
        let estado = estado().await;
        let s = editor(&estado);
        let app = router(estado.clone());
        let sin_cabecera = Request::post("/consumos/1")
            .header(header::COOKIE, &s)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from("fecha=2026-10-27&cantidad_1=2"))
            .unwrap();
        let respuesta = enviar(&app, sin_cabecera).await;
        assert_eq!(respuesta.status(), StatusCode::FORBIDDEN);
        assert!(
            texto(respuesta)
                .await
                .contains("Token de seguridad expirado")
        );

        let otro_token = Request::post("/consumos/1")
            .header(header::COOKIE, &s)
            .header(csrf::CABECERA, "de-otro-navegador")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from("fecha=2026-10-27&cantidad_1=2"))
            .unwrap();
        assert_eq!(
            enviar(&app, otro_token).await.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            valor::<i64>(&estado, "SELECT COUNT(*) FROM consumos").await,
            0
        );
    }

    #[tokio::test]
    async fn sin_permiso_de_edicion_solo_se_registran_consumos() {
        let estado = estado().await;
        let kiosco = cookies(&estado, false);
        let app = router(estado.clone());

        for ruta in [
            "/registro",
            "/registro/menor",
            "/resumen/menor",
            "/consumos/1",
            "/productos",
        ] {
            let respuesta = enviar(&app, get(&kiosco, ruta)).await;
            assert_eq!(respuesta.status(), StatusCode::OK, "{ruta}");
        }
        for ruta in [
            "/",
            "/pagos/1",
            "/comprobante/1",
            "/cobranza",
            "/cierre",
            "/estudiantes",
        ] {
            let respuesta = enviar(&app, get(&kiosco, ruta)).await;
            assert_eq!(respuesta.status(), StatusCode::SEE_OTHER, "{ruta}");
            assert_eq!(
                cabecera(&respuesta, header::LOCATION),
                "/registro",
                "{ruta}"
            );
        }

        let consumo = enviar(
            &app,
            post(
                &kiosco,
                "/consumos/1",
                "fecha=2026-10-27&cantidad_1=1".to_owned(),
            ),
        )
        .await;
        assert_eq!(consumo.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            valor::<i64>(&estado, "SELECT COUNT(*) FROM consumos").await,
            1
        );

        let producto = enviar(
            &app,
            post(&kiosco, "/productos", "nombre=Nuevo&precio=1.00".to_owned()),
        )
        .await;
        assert_eq!(cabecera(&producto, header::LOCATION), "/registro");
        assert_eq!(
            valor::<i64>(&estado, "SELECT COUNT(*) FROM productos").await,
            3
        );
    }

    #[tokio::test]
    async fn una_sesion_invalida_se_borra_y_salir_cierra_la_sesion() {
        let app = router(estado().await);
        let falsa = enviar(&app, get("kiosco_session=falsa.firma", "/")).await;
        assert_eq!(cabecera(&falsa, header::LOCATION), "/login");
        assert!(
            set_cookies(&falsa)
                .iter()
                .any(|c| c.starts_with("kiosco_session=;") && c.contains("Max-Age=0"))
        );

        let salir = enviar(&app, get(&anonimo(), "/logout")).await;
        assert_eq!(salir.status(), StatusCode::SEE_OTHER);
        assert_eq!(cabecera(&salir, header::LOCATION), "/login");
        assert!(
            set_cookies(&salir)
                .iter()
                .any(|c| c.starts_with("kiosco_session=;") && c.contains("Max-Age=0"))
        );
    }
}
