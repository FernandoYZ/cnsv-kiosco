//! Edit permission: what a user without it can and cannot do.

use super::*;

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

// ---- Regression tests from the independent auth verification ----
const RUTAS_GET_SESION: [&str; 5] = [
    "/registro",
    "/registro/menor",
    "/resumen/menor",
    "/consumos/1",
    "/productos",
];

const RUTAS_GET_EDICION: [&str; 6] = [
    "/",
    "/pagos/1",
    "/comprobante/1",
    "/cobranza",
    "/cierre",
    "/estudiantes",
];

#[tokio::test]
async fn matriz_de_permisos_get_y_post() {
    let estado = estado().await;
    let editor_c = cookies(&estado, true);
    let kiosco = cookies(&estado, false);
    let app = router(estado.clone());

    for ruta in RUTAS_GET_SESION.iter().chain(RUTAS_GET_EDICION.iter()) {
        let r = enviar(&app, get(&editor_c, ruta)).await;
        assert_eq!(r.status(), StatusCode::OK, "editor GET {ruta}");
        let r = enviar(&app, get(&anonimo(), ruta)).await;
        assert_eq!(redirige_a(&r), Some("/login"), "anon GET {ruta}");
    }
    for ruta in RUTAS_GET_EDICION {
        let r = enviar(&app, get(&kiosco, ruta)).await;
        assert_eq!(redirige_a(&r), Some("/registro"), "kiosco GET {ruta}");
    }

    let antes = cambios(&estado).await;
    for ruta in RUTAS_POST_EDICION {
        let r = enviar(&app, post(&kiosco, ruta, CUERPO.to_owned())).await;
        assert_eq!(r.status(), StatusCode::SEE_OTHER, "kiosco POST {ruta}");
        assert_eq!(redirige_a(&r), Some("/registro"), "kiosco POST {ruta}");
    }
    for ruta in RUTAS_POST_EDICION.iter().chain(["/consumos/1"].iter()) {
        let r = enviar(&app, post(&anonimo(), ruta, CUERPO.to_owned())).await;
        assert_eq!(redirige_a(&r), Some("/login"), "anon POST {ruta}");
    }
    assert_eq!(
        cambios(&estado).await,
        antes,
        "rejected POSTs write nothing"
    );

    // Editor POSTs reach the handlers (never bounced by auth).
    for ruta in RUTAS_POST_EDICION.iter().chain(["/consumos/1"].iter()) {
        let r = enviar(&app, post(&editor_c, ruta, CUERPO.to_owned())).await;
        assert_ne!(r.status(), StatusCode::FORBIDDEN, "editor POST {ruta}");
        let destino = redirige_a(&r).unwrap_or("");
        assert!(
            destino != "/login" && destino != "/registro" || ruta == &"/consumos/1",
            "editor POST {ruta} -> {destino}"
        );
    }
}

#[tokio::test]
async fn sin_permiso_de_edicion_no_hay_enlaces_a_inicio() {
    let estado = estado().await;
    let kiosco = cookies(&estado, false);
    let jenny = editor(&estado);
    let app = router(estado);

    for ruta in ["/registro", "/productos"] {
        let html = texto(enviar(&app, get(&kiosco, ruta)).await).await;
        assert!(!html.contains(r#"href="/""#), "{ruta} links to Inicio");
        assert!(html.contains(r#"action="/logout""#) || ruta != "/registro");
    }
    let html = texto(enviar(&app, get(&jenny, "/registro")).await).await;
    assert!(
        html.contains(r#"href="/""#),
        "editors keep the Inicio button"
    );
}
