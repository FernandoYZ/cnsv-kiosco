//! Payments and receipts: no duplicate payments, CSRF, navigation back.

use super::*;

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
