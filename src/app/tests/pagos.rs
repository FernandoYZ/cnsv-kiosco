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

/// Student 1 pays S/ 2.00 in the week of Monday 2026-10-12 without consuming
/// anything: that week ends S/ 2.00 in their favor, and the next week starts
/// with it as previous balance.
async fn con_saldo_a_favor() -> AppState {
    let estado = estado().await;
    valor::<i64>(
        &estado,
        "INSERT INTO pagos (id_estudiante, monto, fecha_pago) VALUES (1, 2.00, '2026-10-14') RETURNING id_pago",
    )
    .await;
    estado
}

#[tokio::test]
async fn el_comprobante_muestra_saldo_a_favor_en_vez_de_un_total_negativo() {
    let estado = con_saldo_a_favor().await;
    let s = editor(&estado);
    let html = texto(
        enviar(
            &router(estado),
            get(&s, "/comprobante/1?fecha=2026-10-12&grado=3"),
        )
        .await,
    )
    .await;
    assert!(html.contains("Saldo a favor:"));
    assert!(!html.contains("Total a pagar:"));
    assert!(!html.contains("S/ -2.00"));
}

#[tokio::test]
async fn el_comprobante_muestra_el_saldo_a_favor_anterior_como_descuento() {
    let estado = con_saldo_a_favor().await;
    let s = editor(&estado);
    let html = texto(
        enviar(
            &router(estado),
            get(&s, "/comprobante/1?fecha=2026-10-19&grado=3"),
        )
        .await,
    )
    .await;
    assert!(html.contains("Saldo a favor anterior:"));
    assert!(!html.contains("Deuda anterior:"));
    assert!(html.contains("- S/ 2.00"));
    assert!(!html.contains("S/ -2.00"));
}

#[tokio::test]
async fn la_pagina_de_pagos_muestra_el_saldo_a_favor_sin_signo_negativo() {
    let estado = con_saldo_a_favor().await;
    let s = editor(&estado);
    let html = texto(
        enviar(
            &router(estado),
            get(&s, "/pagos/1?fecha=2026-10-19&grado=3"),
        )
        .await,
    )
    .await;
    assert!(html.contains("a favor"));
    assert!(html.contains("Saldo a favor anterior"));
    assert!(!html.contains("S/ -2.00"));
}
