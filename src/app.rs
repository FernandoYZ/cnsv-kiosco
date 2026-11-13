use axum::{
    Router,
    extract::State,
    http::{HeaderValue, header},
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
    assets, cierre, cobranza, consumos, dashboard, db, error::AppError, estudiantes, pagos,
    productos, state::AppState,
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(dashboard::handler::index))
        // Consumption
        .route("/registro", get(consumos::handler::inicio))
        .route("/registro/{sector}", get(consumos::handler::sector))
        .route("/resumen/{sector}", get(consumos::handler::resumen))
        .route(
            "/consumos/{id_estudiante}",
            get(consumos::handler::editor).post(consumos::handler::guardar),
        )
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
        .route(
            "/productos",
            get(productos::handler::listar).post(productos::handler::crear),
        )
        .route("/productos/{id}", post(productos::handler::actualizar))
        .route(
            "/productos/{id}/estado",
            post(productos::handler::cambiar_estado),
        )
        .route("/health", get(health))
        .fallback(|| async { AppError::NotFound })
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
    use crate::test_support::{estado, valor};

    async fn enviar(app: &Router, request: Request<Body>) -> Response {
        app.clone().oneshot(request).await.unwrap()
    }

    fn get(uri: &str) -> Request<Body> {
        Request::get(uri).body(Body::empty()).unwrap()
    }

    fn post(uri: &str, cuerpo: String) -> Request<Body> {
        Request::post(uri)
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
        let app = router(estado.clone());
        let html = texto(enviar(&app, get("/pagos/1?fecha=2026-10-26&grado=3")).await).await;
        let token = extraer(&html, r#"name="token" value=""#).to_owned();
        let cuerpo = format!("monto=5.00&fecha_pago=2026-10-26&grado=3&token={token}");

        let primero = enviar(&app, post("/pagos/1", cuerpo.clone())).await;
        assert_eq!(primero.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            cabecera(&primero, header::LOCATION),
            "/pagos/1?fecha=2026-10-26&grado=3&ok=pago"
        );

        let segundo = enviar(&app, post("/pagos/1", cuerpo)).await;
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
        let app = router(estado.clone());
        let respuesta = enviar(
            &app,
            post("/pagos/1", "monto=5&fecha_pago=2026-10-26".to_owned()),
        )
        .await;
        assert!(respuesta.status().is_client_error());
        assert_eq!(valor::<i64>(&estado, "SELECT COUNT(*) FROM pagos").await, 0);
    }

    #[tokio::test]
    async fn guardar_un_consumo_vuelve_a_la_lista_del_sector() {
        let estado = estado().await;
        let app = router(estado.clone());
        let respuesta = enviar(
            &app,
            post(
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
        let app = router(estado().await);
        let pagina = enviar(&app, get("/?grado=3&fecha=2026-10-27")).await;
        assert_eq!(cabecera(&pagina, header::CACHE_CONTROL), "no-store");

        let html = texto(pagina).await;
        let css = format!(
            "/public/css/app.css?v={}",
            extraer(&html, "/public/css/app.css?v=")
        );
        let estatico = enviar(&app, get(&css)).await;
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
        let app = router(estado);

        let cobranza = texto(enviar(&app, get("/cobranza")).await).await;
        assert!(cobranza.contains("Alva, Ana"));
        assert!(cobranza.contains("S/ 6.00"), "10.00 consumed - 4.00 paid");
        assert!(cobranza.contains("tiene un saldo pendiente de S/ 6.00"));
        assert!(
            !cobranza.contains("Bravo, Luis"),
            "students without debt are not listed"
        );

        let cierre = texto(enviar(&app, get("/cierre?fecha=2026-10-27")).await).await;
        assert!(cierre.contains("S/ 10.00"), "sold");
        assert!(cierre.contains("S/ 4.00"), "collected");
    }

    #[tokio::test]
    async fn el_comprobante_abierto_desde_cobranza_vuelve_a_cobranza() {
        let app = router(estado().await);
        let html = texto(
            enviar(
                &app,
                get("/comprobante/1?fecha=2026-10-26&grado=3&desde=cobranza"),
            )
            .await,
        )
        .await;
        assert!(html.contains(r#"href="/cobranza""#));
    }

    #[tokio::test]
    async fn rutas_desconocidas_responden_404() {
        let app = router(estado().await);
        assert_eq!(
            enviar(&app, get("/no-existe")).await.status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            enviar(&app, get("/public/no-existe.css")).await.status(),
            StatusCode::NOT_FOUND
        );
    }
}
