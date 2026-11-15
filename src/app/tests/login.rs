//! Login, sessions and the failed-login limit.

use super::*;

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
    assert!(
        html.contains(
            "Has realizado demasiados intentos de login. Intenta de nuevo en 15 minutos."
        )
    );
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
        html.contains("El token de seguridad ha expirado. Recarga la página e intenta de nuevo.")
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

    let salir = enviar(&app, post(&anonimo(), "/logout", String::new())).await;
    assert_eq!(salir.status(), StatusCode::SEE_OTHER);
    assert_eq!(cabecera(&salir, header::LOCATION), "/login");
    assert!(
        set_cookies(&salir)
            .iter()
            .any(|c| c.starts_with("kiosco_session=;") && c.contains("Max-Age=0"))
    );
}

#[tokio::test]
async fn login_csrf_errado_y_sin_cookie() {
    let app = router(estado().await);
    let errado = Request::post("/login")
        .header(header::COOKIE, anonimo())
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "csrf_token=otro&usuario=jenny&password={CLAVE}"
        )))
        .unwrap();
    let r = enviar(&app, errado).await;
    assert_eq!(redirige_a(&r), Some("/login?error=csrf"));
    assert!(
        !set_cookies(&r)
            .iter()
            .any(|c| c.starts_with("kiosco_session="))
    );

    // First visit without any CSRF cookie: rejected, and a cookie is issued.
    let sin_cookie = Request::post("/login")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(csrf::CABECERA, CSRF)
        .body(Body::from(format!("usuario=jenny&password={CLAVE}")))
        .unwrap();
    let r = enviar(&app, sin_cookie).await;
    assert_eq!(redirige_a(&r), Some("/login?error=csrf"));
    assert!(
        set_cookies(&r)
            .iter()
            .any(|c| c.starts_with("kiosco_csrf="))
    );
}

#[tokio::test]
async fn tokens_de_sesion_alterados_vencidos_o_de_otra_llave() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let estado = estado().await;
    let app = router(estado.clone());
    let ahora = chrono::Utc::now().timestamp();
    let kiosco = Sesion {
        id_usuario: 2,
        puede_editar: false,
    };

    let valido = estado.auth.llave.firmar(kiosco, ahora);
    let (datos, firma) = valido.split_once('.').unwrap();
    let datos = String::from_utf8(URL_SAFE_NO_PAD.decode(datos).unwrap()).unwrap();
    let ascendido = format!(
        "{}.{firma}",
        URL_SAFE_NO_PAD.encode(datos.replacen(":0:", ":1:", 1))
    );
    let vencido = estado
        .auth
        .llave
        .firmar(kiosco, ahora - sesion::DURACION_SEGUNDOS - 1);
    let otra_llave = crate::auth::sesion::Llave::aleatoria().firmar(kiosco, ahora);

    for (caso, token) in [
        ("alterado", ascendido),
        ("vencido", vencido),
        ("otra llave", otra_llave),
        ("sin firma", valido.split('.').next().unwrap().to_owned()),
    ] {
        let c = format!("{}={token}; {}", sesion::COOKIE, anonimo());
        for ruta in ["/", "/registro"] {
            let r = enviar(&app, get(&c, ruta)).await;
            assert_eq!(redirige_a(&r), Some("/login"), "{caso} {ruta}");
            assert!(
                set_cookies(&r)
                    .iter()
                    .any(|c| c.starts_with("kiosco_session=;") && c.contains("Max-Age=0")),
                "{caso} clears the cookie"
            );
        }
        // An invalid session does not count as logged in on /login.
        let r = enviar(&app, get(&c, "/login")).await;
        assert_eq!(r.status(), StatusCode::OK, "{caso}");
    }

    // Non-editor login lands on / and is sent to /registro.
    let r = enviar(&app, login("kiosco", CLAVE)).await;
    assert_eq!(redirige_a(&r), Some("/"));
    let sesion_c = set_cookies(&r)
        .into_iter()
        .find(|c| c.starts_with("kiosco_session="))
        .unwrap();
    let valor_c = sesion_c.split(';').next().unwrap().to_owned();
    let r = enviar(&app, get(&format!("{valor_c}; {}", anonimo()), "/")).await;
    assert_eq!(redirige_a(&r), Some("/registro"));
}

fn login_desde(xff: &str, clave: &str) -> Request<Body> {
    let mut r = login("jenny", clave);
    r.headers_mut()
        .insert("x-forwarded-for", xff.parse().unwrap());
    r
}

#[tokio::test]
async fn limite_por_ip_xff_y_reinicio() {
    let app = router(estado().await);
    // Rotating the forged FIRST value does not escape the limit.
    for i in 0..5 {
        let r = enviar(
            &app,
            login_desde(&format!("203.0.113.{i}, 10.0.0.9"), "mal"),
        )
        .await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    }
    let r = enviar(&app, login_desde("198.51.100.1, 10.0.0.9", CLAVE)).await;
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    // Another client IP is not affected.
    let r = enviar(&app, login_desde("10.0.0.10", CLAVE)).await;
    assert_eq!(r.status(), StatusCode::SEE_OTHER);

    // Success resets: 4 failures, success, 4 failures → 5th attempt still allowed.
    for _ in 0..4 {
        enviar(&app, login_desde("10.0.0.11", "mal")).await;
    }
    assert_eq!(
        enviar(&app, login_desde("10.0.0.11", CLAVE)).await.status(),
        StatusCode::SEE_OTHER
    );
    for _ in 0..4 {
        enviar(&app, login_desde("10.0.0.11", "mal")).await;
    }
    assert_eq!(
        enviar(&app, login_desde("10.0.0.11", CLAVE)).await.status(),
        StatusCode::SEE_OTHER
    );

    // Without a proxy, the peer address is used.
    for _ in 0..5 {
        let mut r = login("jenny", "mal");
        r.extensions_mut().insert(axum::extract::ConnectInfo(
            "192.0.2.1:5000".parse::<std::net::SocketAddr>().unwrap(),
        ));
        enviar(&app, r).await;
    }
    let mut r = login("jenny", CLAVE);
    r.extensions_mut().insert(axum::extract::ConnectInfo(
        "192.0.2.1:6000".parse::<std::net::SocketAddr>().unwrap(),
    ));
    assert_eq!(
        enviar(&app, r).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn cookie_secure_configurable() {
    let mut estado = estado().await;
    estado.auth = crate::auth::Auth::new(true);
    let app = router(estado);
    let r = enviar(&app, login("jenny", CLAVE)).await;
    let c = set_cookies(&r);
    assert!(
        c.iter()
            .any(|c| c.starts_with("kiosco_session=") && c.contains("; Secure")),
        "{c:?}"
    );
    let pagina = enviar(&app, get("", "/login")).await;
    assert!(
        set_cookies(&pagina)
            .iter()
            .any(|c| c.starts_with("kiosco_csrf=") && c.contains("Secure"))
    );
}

#[tokio::test]
async fn login_con_caracteres_especiales_codificados() {
    let estado = estado().await;
    let clave = r#"ab%cd@"e=f&g"#;
    let hash = crate::auth::password::generar(clave, &[9; 16]);
    estado
        .db
        .call(move |c| {
            c.execute("INSERT INTO usuarios VALUES (3, 'especial', ?1, 0)", [hash])?;
            Ok::<_, tokio_rusqlite::rusqlite::Error>(())
        })
        .await
        .unwrap();
    let app = router(estado);
    let codificada = "ab%25cd%40%22e%3Df%26g";
    let r = enviar(&app, login("especial", codificada)).await;
    assert_eq!(redirige_a(&r), Some("/"));
}
