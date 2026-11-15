//! CSRF protection on every POST.

use super::*;

#[tokio::test]
async fn el_token_csrf_tambien_vale_como_campo_del_formulario() {
    let app = router(estado().await);
    let campo = Request::post("/login")
        .header(header::COOKIE, anonimo())
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "csrf_token={CSRF}&usuario=jenny&password={CLAVE}"
        )))
        .unwrap();
    let respuesta = enviar(&app, campo).await;
    assert_eq!(cabecera(&respuesta, header::LOCATION), "/");
}

#[tokio::test]
async fn un_post_con_sesion_pero_sin_csrf_no_escribe() {
    let estado = estado().await;
    let s = editor(&estado);
    let app = router(estado.clone());
    let sin_cabecera = Request::post("/consumos/1")
        .header(header::COOKIE, &s)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from("fecha=2026-10-27&cantidad_1=2"))
        .unwrap();
    let respuesta = enviar(&app, sin_cabecera).await;
    assert_eq!(respuesta.status(), StatusCode::FORBIDDEN);
    assert!(
        texto(respuesta)
            .await
            .contains("Token de seguridad expirado")
    );

    let otro_token = Request::post("/consumos/1")
        .header(header::COOKIE, &s)
        .header(csrf::CABECERA, "de-otro-navegador")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from("fecha=2026-10-27&cantidad_1=2"))
        .unwrap();
    assert_eq!(
        enviar(&app, otro_token).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        valor::<i64>(&estado, "SELECT COUNT(*) FROM consumos").await,
        0
    );
}

#[tokio::test]
async fn csrf_en_cada_post_protegido_no_escribe() {
    let estado = estado().await;
    let s = editor(&estado);
    let solo_sesion = s.split("; ").next().unwrap().to_owned();
    let app = router(estado.clone());
    let antes = cambios(&estado).await;

    for ruta in RUTAS_POST_EDICION.iter().chain(["/consumos/1"].iter()) {
        let casos: Vec<(&str, String, Option<&str>, String)> = vec![
            ("sin token", s.clone(), None, CUERPO.to_owned()),
            (
                "cabecera errada",
                s.clone(),
                Some("otro"),
                CUERPO.to_owned(),
            ),
            ("cabecera vacia", s.clone(), Some(""), CUERPO.to_owned()),
            (
                "solo cookie, campo vacio",
                s.clone(),
                None,
                format!("csrf_token=&{CUERPO}"),
            ),
            (
                "campo errado",
                s.clone(),
                None,
                format!("csrf_token=otro&{CUERPO}"),
            ),
            (
                "sin cookie csrf",
                solo_sesion.clone(),
                Some(CSRF),
                CUERPO.to_owned(),
            ),
            (
                "cookie csrf vacia",
                format!("{solo_sesion}; {}=", csrf::COOKIE),
                Some(""),
                format!("csrf_token=&{CUERPO}"),
            ),
            (
                "cabecera errada gana al campo correcto",
                s.clone(),
                Some("otro"),
                format!("csrf_token={CSRF}&{CUERPO}"),
            ),
        ];
        for (caso, cookie, cabecera_csrf, cuerpo) in casos {
            let mut b = Request::post(*ruta)
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
            if let Some(t) = cabecera_csrf {
                b = b.header(csrf::CABECERA, t);
            }
            let r = enviar(&app, b.body(Body::from(cuerpo)).unwrap()).await;
            assert_eq!(r.status(), StatusCode::FORBIDDEN, "{ruta}: {caso}");
        }
    }
    assert_eq!(
        cambios(&estado).await,
        antes,
        "no writes after CSRF rejections"
    );

    // The token as a form field is accepted (fallback submit of app.js).
    let campo = Request::post("/consumos/1")
        .header(header::COOKIE, &s)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "fecha=2026-10-27&cantidad_1=1&csrf_token={CSRF}"
        )))
        .unwrap();
    assert_eq!(enviar(&app, campo).await.status(), StatusCode::SEE_OTHER);

    // Oversized body without header → 400, nothing written.
    let antes = cambios(&estado).await;
    let grande = Request::post("/consumos/1")
        .header(header::COOKIE, &s)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "csrf_token={CSRF}&x={}",
            "a".repeat(70_000)
        )))
        .unwrap();
    assert_eq!(enviar(&app, grande).await.status(), StatusCode::BAD_REQUEST);
    assert_eq!(cambios(&estado).await, antes);
}
