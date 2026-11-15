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
