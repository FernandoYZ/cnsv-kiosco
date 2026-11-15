//! Logout: POST only, with a valid CSRF token.

use super::*;

/// True when the response deletes the session cookie (logs the user out).
fn sesion_borrada(respuesta: &Response) -> bool {
    set_cookies(respuesta)
        .iter()
        .any(|c| c.starts_with("kiosco_session=;") && c.contains("Max-Age=0"))
}

/// POST /logout with the session cookie and an optional CSRF header.
fn salir_con_token(cookies: &str, token: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri("/logout")
        .header(header::COOKIE, cookies)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    if let Some(token) = token {
        request = request.header(csrf::CABECERA, token);
    }
    request.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn salir_por_get_no_cierra_la_sesion() {
    // A link or an <img src="/logout"> on another site must not log anyone out.
    let estado = estado().await;
    let s = editor(&estado);
    let respuesta = enviar(&router(estado), get(&s, "/logout")).await;
    assert_eq!(respuesta.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert!(!sesion_borrada(&respuesta));
}

#[tokio::test]
async fn salir_sin_token_csrf_no_cierra_la_sesion() {
    let estado = estado().await;
    let s = editor(&estado);
    let respuesta = enviar(&router(estado), salir_con_token(&s, None)).await;
    assert_eq!(respuesta.status(), StatusCode::FORBIDDEN);
    assert!(!sesion_borrada(&respuesta));
}

#[tokio::test]
async fn salir_con_token_csrf_incorrecto_no_cierra_la_sesion() {
    let estado = estado().await;
    let s = editor(&estado);
    let respuesta = enviar(&router(estado), salir_con_token(&s, Some("otro-token"))).await;
    assert_eq!(respuesta.status(), StatusCode::FORBIDDEN);
    assert!(!sesion_borrada(&respuesta));
}

#[tokio::test]
async fn salir_con_token_csrf_cierra_la_sesion_y_vuelve_al_login() {
    let estado = estado().await;
    let s = editor(&estado);
    let respuesta = enviar(&router(estado), salir_con_token(&s, Some(CSRF))).await;
    assert_eq!(redirige_a(&respuesta), Some("/login"));
    assert!(sesion_borrada(&respuesta));
}
