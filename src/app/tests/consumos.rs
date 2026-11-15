//! Consumption registration.

use super::*;

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
async fn guardar_sin_sector_ni_grado_vuelve_a_la_lista_del_sector_del_estudiante() {
    // A cashier cannot open Inicio: without sector/grado the editor must not send them there.
    let estado = estado().await;
    let kiosco = cookies(&estado, false);
    let app = router(estado);
    for (estudiante, destino) in [
        (1, "/registro/menor?fecha=2026-10-27&ok=consumo"),
        (2, "/registro/mayor?fecha=2026-10-27&ok=consumo"),
    ] {
        let uri = format!("/consumos/{estudiante}");
        let cuerpo = "fecha=2026-10-27&cantidad_1=1".to_owned();
        let respuesta = enviar(&app, post(&kiosco, &uri, cuerpo)).await;
        assert_eq!(cabecera(&respuesta, header::LOCATION), destino, "{uri}");
    }
}

#[tokio::test]
async fn guardar_desde_la_tabla_semanal_vuelve_a_la_tabla() {
    let estado = estado().await;
    let s = editor(&estado);
    let cuerpo = "fecha=2026-10-27&grado=3&cantidad_1=1".to_owned();
    let respuesta = enviar(&router(estado), post(&s, "/consumos/1", cuerpo)).await;
    assert_eq!(
        cabecera(&respuesta, header::LOCATION),
        "/?fecha=2026-10-27&grado=3&ok=consumo"
    );
}

#[tokio::test]
async fn cancelar_en_el_editor_sin_sector_vuelve_a_la_lista_del_sector_del_estudiante() {
    let estado = estado().await;
    let kiosco = cookies(&estado, false);
    let html = texto(
        enviar(
            &router(estado),
            get(&kiosco, "/consumos/1?fecha=2026-10-27"),
        )
        .await,
    )
    .await;
    assert!(
        html.contains(r#"href="/registro/menor?fecha=2026-10-27""#),
        "cancel link"
    );
}
