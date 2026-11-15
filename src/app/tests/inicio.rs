//! Weekly table on the home page.

use super::*;

/// On phones the header row scrolls out of view, so each consumption cell
/// carries its own day label.
#[tokio::test]
async fn cada_celda_de_consumo_muestra_su_dia() {
    let estado = estado().await;
    let s = editor(&estado);
    let html = texto(enviar(&router(estado), get(&s, "/?fecha=2026-10-26&grado=3")).await).await;

    for (fecha, dia) in [
        ("2026-10-26", "Lun 26/10"),
        ("2026-10-27", "Mar 27/10"),
        ("2026-10-28", "Mié 28/10"),
        ("2026-10-29", "Jue 29/10"),
        ("2026-10-30", "Vie 30/10"),
        ("2026-10-31", "Sáb 31/10"),
    ] {
        let enlace = format!(r#"href="/consumos/1?fecha={fecha}&amp;grado=3""#);
        let celda = html
            .split_once(&enlace)
            .unwrap_or_else(|| panic!("no cell for {fecha}"))
            .1
            .split_once("</a>")
            .unwrap()
            .0;
        assert!(celda.contains(dia), "cell {fecha} has no «{dia}»");
    }
}

#[tokio::test]
async fn subtotal_y_deuda_anterior_en_cero_no_se_resaltan() {
    let estado = estado().await;
    let s = editor(&estado);
    let html = texto(enviar(&router(estado), get(&s, "/?fecha=2026-10-26&grado=3")).await).await;
    assert!(html.contains("Alva, Ana"));
    assert!(
        !html.contains("bg-orange-50"),
        "zero amounts must not look like debt"
    );
}

#[tokio::test]
async fn subtotal_con_consumos_se_resalta() {
    let estado = estado().await;
    valor::<i64>(
        &estado,
        "INSERT INTO consumos (id_estudiante, id_producto, cantidad, precio_unitario_venta, fecha_consumo) VALUES (1, 1, 1, 1.50, '2026-10-27') RETURNING 1",
    )
    .await;
    let s = editor(&estado);
    let html = texto(enviar(&router(estado), get(&s, "/?fecha=2026-10-26&grado=3")).await).await;
    assert!(html.contains("bg-orange-50"));
}

#[tokio::test]
async fn la_tabla_muestra_el_saldo_a_favor_sin_signo_negativo() {
    let estado = estado().await;
    valor::<i64>(
        &estado,
        "INSERT INTO pagos (id_estudiante, monto, fecha_pago) VALUES (1, 2.00, '2026-10-14') RETURNING id_pago",
    )
    .await;
    let s = editor(&estado);
    let html = texto(enviar(&router(estado), get(&s, "/?fecha=2026-10-19&grado=3")).await).await;
    assert!(html.contains("a favor"));
    assert!(!html.contains("S/ -2.00"));
}

#[tokio::test]
async fn sin_deuda_la_columna_pagos_no_se_resalta() {
    let estado = estado().await;
    let s = editor(&estado);
    let html = texto(enviar(&router(estado), get(&s, "/?fecha=2026-10-26&grado=3")).await).await;
    assert!(html.contains("Alva, Ana"));
    assert!(
        !html.contains("bg-amber-100"),
        "nothing to collect, nothing to highlight"
    );
}

#[tokio::test]
async fn con_deuda_y_sin_pagos_la_columna_pagos_se_resalta() {
    let estado = estado().await;
    valor::<i64>(
        &estado,
        "INSERT INTO consumos (id_estudiante, id_producto, cantidad, precio_unitario_venta, fecha_consumo) VALUES (1, 1, 1, 1.50, '2026-10-27') RETURNING 1",
    )
    .await;
    let s = editor(&estado);
    let html = texto(enviar(&router(estado), get(&s, "/?fecha=2026-10-26&grado=3")).await).await;
    assert!(html.contains("bg-amber-100"));
}
