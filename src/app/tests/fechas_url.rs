//! `?fecha=` in the URL: happy and unhappy paths on every screen.

use super::*;

// ── `?fecha=` in the URL: happy and unhappy paths on every screen ───────
#[rstest::rstest]
#[tokio::test]
async fn una_fecha_invalida_en_la_url_no_rompe_ninguna_pantalla(
    #[values(
        "/",
        "/registro/menor",
        "/resumen/menor",
        "/cierre",
        "/consumos/1",
        "/pagos/1",
        "/comprobante/1"
    )]
    ruta: &str,
    #[values(
        "2026-02-29",
        "Monday",
        "%3Cscript%3Ealert(1)%3C%2Fscript%3E",
        "%27%20OR%201%3D1%20--",
        "262143-12-31",
        "",
        "%F0%9F%93%85"
    )]
    fecha: &str,
) {
    let estado = estado().await;
    let s = editor(&estado);
    let uri = format!("{ruta}?fecha={fecha}");
    let respuesta = enviar(&router(estado), get(&s, &uri)).await;
    assert_eq!(respuesta.status(), StatusCode::OK, "{uri}");
    // The raw value is never reflected into the page.
    assert!(!texto(respuesta).await.contains("<script>alert"), "{uri}");
}

#[tokio::test]
async fn una_fecha_valida_en_la_url_es_la_que_se_muestra() {
    let estado = estado().await;
    let s = editor(&estado);
    let html = texto(enviar(&router(estado), get(&s, "/cierre?fecha=2026-11-11")).await).await;
    assert!(html.contains("miércoles, 11 de noviembre de 2026"));
}

#[tokio::test]
async fn una_fecha_invalida_en_la_url_muestra_el_dia_de_hoy() {
    let estado = estado().await;
    let s = editor(&estado);
    let html = texto(enviar(&router(estado), get(&s, "/cierre?fecha=Monday")).await).await;
    assert!(html.contains(&crate::fechas::larga(crate::fechas::hoy())));
}
