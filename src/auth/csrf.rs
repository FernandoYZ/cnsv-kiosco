//! Double-submit CSRF token: a random cookie that every POST must echo in
//! the `X-CSRF-Token` header (sent by `app.js`) or the `csrf_token` field.

use axum::{
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{Method, header},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use subtle::ConstantTimeEq;

use super::leer_cookie;
use crate::{error::AppError, state::AppState};

pub const COOKIE: &str = "kiosco_csrf";
pub const CAMPO: &str = "csrf_token";
pub const CABECERA: &str = "x-csrf-token";
/// Forms are tiny; anything bigger is not a form of this app.
const MAXIMO_CUERPO: usize = 64 * 1024;

/// Token of the current request, for pages that render a form field.
#[derive(Clone)]
pub struct TokenCsrf(pub String);

fn nuevo_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("operating system random generator");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn coinciden(esperado: &str, recibido: &str) -> bool {
    !esperado.is_empty() && bool::from(esperado.as_bytes().ct_eq(recibido.as_bytes()))
}

/// `csrf_token` from an urlencoded body. Tokens are base64url, so they need
/// no percent-decoding.
fn campo_del_formulario(cuerpo: &[u8]) -> Option<&str> {
    std::str::from_utf8(cuerpo)
        .ok()?
        .split('&')
        .find_map(|par| par.strip_prefix(CAMPO)?.strip_prefix('='))
}

/// Issues the cookie when missing and rejects POSTs whose token does not match.
pub async fn proteger(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let existente = leer_cookie(request.headers(), COOKIE)
        .filter(|t| !t.is_empty())
        .map(str::to_owned);
    let nuevo = existente.is_none();
    let token = existente.clone().unwrap_or_else(nuevo_token);

    let (mut partes, cuerpo) = request.into_parts();
    let mut cuerpo = cuerpo;
    if partes.method == Method::POST {
        let enviado_en_cabecera = partes
            .headers
            .get(CABECERA)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let valido = match (&existente, enviado_en_cabecera) {
            (Some(esperado), Some(enviado)) => coinciden(esperado, &enviado),
            (Some(esperado), None) => {
                let Ok(bytes) = to_bytes(cuerpo, MAXIMO_CUERPO).await else {
                    return AppError::BadRequest("Formulario demasiado grande.".to_owned())
                        .into_response();
                };
                let valido = campo_del_formulario(&bytes).is_some_and(|t| coinciden(esperado, t));
                cuerpo = Body::from(bytes);
                valido
            }
            (None, _) => false,
        };
        if !valido {
            tracing::warn!(path = %partes.uri.path(), "CSRF token rejected");
            let mut respuesta = if partes.uri.path() == "/login" {
                Redirect::to("/login?error=csrf").into_response()
            } else {
                AppError::CsrfInvalido.into_response()
            };
            if nuevo {
                agregar_cookie(&state, &mut respuesta, &token);
            }
            return respuesta;
        }
    }

    partes.extensions.insert(TokenCsrf(token.clone()));
    let mut respuesta = next.run(Request::from_parts(partes, cuerpo)).await;
    if nuevo {
        agregar_cookie(&state, &mut respuesta, &token);
    }
    respuesta
}

fn agregar_cookie(state: &AppState, respuesta: &mut Response, token: &str) {
    // Readable by `app.js` (not HttpOnly) and alive for the browser session.
    let cookie = state.auth.cookie(COOKIE, token, None, false);
    respuesta.headers_mut().append(header::SET_COOKIE, cookie);
}

#[cfg(test)]
mod tests {
    use super::{campo_del_formulario, coinciden, nuevo_token};

    #[test]
    fn los_tokens_son_aleatorios_y_url_safe() {
        let token = nuevo_token();
        assert_eq!(token.len(), 43, "32 bytes in base64url");
        assert!(
            token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        );
        assert_ne!(token, nuevo_token());
    }

    #[test]
    fn compara_tokens_y_lee_el_campo_del_formulario() {
        assert!(coinciden("abc", "abc"));
        assert!(!coinciden("abc", "abd"));
        assert!(!coinciden("abc", "ab"));
        assert!(!coinciden("", ""), "an empty token never matches");

        assert_eq!(campo_del_formulario(b"a=1&csrf_token=xyz&b=2"), Some("xyz"));
        assert_eq!(campo_del_formulario(b"csrf_tokenx=1"), None);
        assert_eq!(campo_del_formulario(b"a=1"), None);
    }
}
