//! Collections (cobranza) and day close (cierre).

use super::*;

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
