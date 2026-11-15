//! HTTP tests of the whole router against an in-memory SQLite (component tests),
//! one file per feature. This module holds the shared helpers.

mod consumos;
mod estaticos;
mod fechas_url;
mod login;
mod logout;
mod pagos;
mod permisos;
mod productos;
mod proteccion_csrf;
mod reportes;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    response::Response,
};

use tower::ServiceExt;

use super::router;

use crate::{
    auth::{
        csrf,
        sesion::{self, Sesion},
    },
    state::AppState,
    test_support::{CLAVE, estado, valor},
};

async fn enviar(app: &Router, request: Request<Body>) -> Response {
    app.clone().oneshot(request).await.unwrap()
}

const CSRF: &str = "token-csrf-de-prueba";

/// Cookies of a logged-in browser (empty string = anonymous).
fn cookies(estado: &AppState, puede_editar: bool) -> String {
    let sesion = Sesion {
        id_usuario: if puede_editar { 1 } else { 2 },
        puede_editar,
    };
    let token = estado
        .auth
        .llave
        .firmar(sesion, chrono::Utc::now().timestamp());
    format!("{}={token}; {}={CSRF}", sesion::COOKIE, csrf::COOKIE)
}

fn editor(estado: &AppState) -> String {
    cookies(estado, true)
}

fn get(cookies: &str, uri: &str) -> Request<Body> {
    Request::get(uri)
        .header(header::COOKIE, cookies)
        .body(Body::empty())
        .unwrap()
}

/// Form POST carrying the CSRF header the way `app.js` sends it.
fn post(cookies: &str, uri: &str, cuerpo: String) -> Request<Body> {
    Request::post(uri)
        .header(header::COOKIE, cookies)
        .header(csrf::CABECERA, CSRF)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(cuerpo))
        .unwrap()
}

async fn texto(respuesta: Response) -> String {
    let bytes = to_bytes(respuesta.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn cabecera(respuesta: &Response, nombre: header::HeaderName) -> &str {
    respuesta.headers()[nombre].to_str().unwrap()
}

fn extraer<'a>(html: &'a str, antes: &str) -> &'a str {
    let inicio = html.find(antes).expect(antes) + antes.len();
    let fin = inicio + html[inicio..].find('"').unwrap();
    &html[inicio..fin]
}

// ---- Login, sessions, CSRF and permissions ----
fn set_cookies(respuesta: &Response) -> Vec<String> {
    respuesta
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().to_owned())
        .collect()
}

/// Anonymous browser that already loaded the login page (has the CSRF cookie).
fn anonimo() -> String {
    format!("{}={CSRF}", csrf::COOKIE)
}

fn login(usuario: &str, clave: &str) -> Request<Body> {
    post(
        &anonimo(),
        "/login",
        format!("usuario={usuario}&password={clave}"),
    )
}

const RUTAS_POST_EDICION: [&str; 8] = [
    "/pagos/1",
    "/pagos/1/1/eliminar",
    "/estudiantes",
    "/estudiantes/1",
    "/estudiantes/1/estado",
    "/productos",
    "/productos/1",
    "/productos/1/estado",
];

const CUERPO: &str = "fecha=2026-10-27&cantidad_1=1&nombre=X&apellido=Y&precio=1.00&monto=1.00&fecha_pago=2026-10-27&grado=3&id_grado=3&token=t";

async fn cambios(estado: &AppState) -> i64 {
    valor::<i64>(estado, "SELECT total_changes()").await
}

fn redirige_a(respuesta: &Response) -> Option<&str> {
    respuesta
        .headers()
        .get(header::LOCATION)
        .map(|v| v.to_str().unwrap())
}
