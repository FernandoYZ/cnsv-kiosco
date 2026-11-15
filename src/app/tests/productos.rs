//! Product catalog page.

use super::*;

/// Markers of the edit controls: the add form, edit links and status forms.
fn tiene_controles_de_edicion(html: &str) -> bool {
    html.contains("Agregar al Catálogo")
        || html.contains("/productos?editar=")
        || html.contains("/estado\"")
}

#[tokio::test]
async fn sin_permiso_de_edicion_la_lista_de_productos_no_muestra_controles_de_edicion() {
    // The server already rejects these POSTs: offering them only bounced the cashier to /registro.
    let estado = estado().await;
    let kiosco = cookies(&estado, false);
    let html = texto(enviar(&router(estado), get(&kiosco, "/productos")).await).await;
    assert!(html.contains("Gelatina"), "products are listed");
    assert!(!tiene_controles_de_edicion(&html), "no edit controls");
}

#[tokio::test]
async fn con_permiso_de_edicion_la_lista_de_productos_muestra_los_controles() {
    let estado = estado().await;
    let s = editor(&estado);
    let html = texto(enviar(&router(estado), get(&s, "/productos")).await).await;
    assert!(html.contains("Agregar al Catálogo"));
    assert!(html.contains("/productos?editar=1"));
    assert!(html.contains("/productos/1/estado"));
}
