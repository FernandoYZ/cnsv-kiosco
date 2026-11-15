//! Static assets, caching, 404 and the installable PWA.

use super::*;

#[tokio::test]
async fn las_paginas_no_se_cachean_y_los_estaticos_versionados_si() {
    let estado = estado().await;
    let s = editor(&estado);
    let app = router(estado);
    let pagina = enviar(&app, get(&s, "/?grado=3&fecha=2026-10-27")).await;
    assert_eq!(cabecera(&pagina, header::CACHE_CONTROL), "no-store");

    let html = texto(pagina).await;
    let css = format!(
        "/public/css/app.css?v={}",
        extraer(&html, "/public/css/app.css?v=")
    );
    let estatico = enviar(&app, get(&s, &css)).await;
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
async fn rutas_desconocidas_responden_404() {
    let estado = estado().await;
    let s = editor(&estado);
    let app = router(estado);
    assert_eq!(
        enviar(&app, get(&s, "/no-existe")).await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        enviar(&app, get(&s, "/public/no-existe.css"))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn publicos_estaticos_404_y_logout() {
    let estado = estado().await;
    let s = editor(&estado);
    let app = router(estado.clone());
    let js = enviar(&app, get("", "/public/js/app.js")).await;
    assert_eq!(js.status(), StatusCode::OK);
    assert!(set_cookies(&js).is_empty(), "no Set-Cookie on assets");
    let cc = js
        .headers()
        .get(header::CACHE_CONTROL)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    assert!(!cc.contains("no-store"), "{cc}");

    assert_eq!(
        enviar(&app, get("", "/health")).await.status(),
        StatusCode::OK
    );
    for c in ["", s.as_str()] {
        assert_eq!(
            enviar(&app, get(c, "/no-existe")).await.status(),
            StatusCode::NOT_FOUND
        );
    }
    let salir = enviar(&app, post(&s, "/logout", String::new())).await;
    assert_eq!(redirige_a(&salir), Some("/login"));
    let borrada = set_cookies(&salir)
        .into_iter()
        .find(|c| c.starts_with("kiosco_session="))
        .unwrap();
    assert!(
        borrada.contains("Max-Age=0") && borrada.contains("HttpOnly"),
        "{borrada}"
    );

    for ruta in ["/", "/registro"] {
        let html = texto(enviar(&app, get(&s, ruta)).await).await;
        assert!(
            html.contains(r#"action="/logout""#),
            "logout form on {ruta}"
        );
    }
}

#[tokio::test]
async fn la_pwa_es_instalable_con_service_worker_e_iconos_png() {
    let app = router(estado().await);
    let sw = enviar(&app, get("", "/sw.js")).await;
    assert_eq!(sw.status(), StatusCode::OK, "public, no login needed");
    assert!(cabecera(&sw, header::CONTENT_TYPE).contains("javascript"));
    assert_eq!(cabecera(&sw, header::CACHE_CONTROL), "no-cache");
    assert!(sw.headers().get(header::SET_COOKIE).is_none());
    assert!(texto(sw).await.contains("addEventListener('fetch'"));

    let manifest = texto(enviar(&app, get("", "/public/manifest.json")).await).await;
    for icono in ["icon-192.png", "icon-512.png", "icon-maskable-512.png"] {
        assert!(manifest.contains(icono), "{icono}");
        let png = enviar(&app, get("", &format!("/public/images/{icono}"))).await;
        assert_eq!(png.status(), StatusCode::OK, "{icono}");
        assert_eq!(cabecera(&png, header::CONTENT_TYPE), "image/png");
    }
    assert!(manifest.contains(r#""purpose": "maskable""#));
}
