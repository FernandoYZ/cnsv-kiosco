//! Login, sessions, CSRF and the edit permission, with the same behaviour as
//! the Go app on the same `usuarios` table:
//!
//! - passwords: argon2id PHC strings, parameters taken from each hash;
//! - session: stateless cookie signed with a key created at startup, so a
//!   restart logs everyone out;
//! - CSRF: double-submit cookie checked on every POST;
//! - `puede_editar = 0` users only reach the consumption screens.

pub mod csrf;
pub mod handler;
pub mod limite;
pub mod middleware;
pub mod password;
pub mod repository;
pub mod sesion;

use axum::http::{HeaderMap, HeaderValue, header};

use self::{limite::LimiteIntentos, sesion::Llave};

/// Auth part of the shared state.
#[derive(Clone)]
pub struct Auth {
    pub llave: Llave,
    /// Adds `Secure` to cookies (production behind HTTPS).
    pub cookie_segura: bool,
    pub intentos: LimiteIntentos,
}

impl Auth {
    pub fn new(cookie_segura: bool) -> Self {
        Self {
            llave: Llave::aleatoria(),
            cookie_segura,
            intentos: LimiteIntentos::default(),
        }
    }

    /// `Set-Cookie` value. `max_age = None` → browser-session cookie; `Some(0)` deletes it.
    pub fn cookie(
        &self,
        nombre: &str,
        valor: &str,
        max_age: Option<i64>,
        http_only: bool,
    ) -> HeaderValue {
        let mut texto = format!("{nombre}={valor}; Path=/; SameSite=Lax");
        if let Some(segundos) = max_age {
            texto.push_str(&format!("; Max-Age={segundos}"));
        }
        if http_only {
            texto.push_str("; HttpOnly");
        }
        if self.cookie_segura {
            texto.push_str("; Secure");
        }
        HeaderValue::from_str(&texto).expect("cookie values are ASCII tokens")
    }
}

/// Value of the cookie `nombre` sent by the browser.
pub fn leer_cookie<'a>(headers: &'a HeaderMap, nombre: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|par| {
            let (clave, valor) = par.trim().split_once('=')?;
            (clave == nombre).then_some(valor)
        })
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header};

    use super::{Auth, leer_cookie};

    #[test]
    fn lee_la_cookie_pedida_entre_varias() {
        let mut headers = HeaderMap::new();
        headers.append(
            header::COOKIE,
            HeaderValue::from_static("a=1; kiosco_csrf=xyz"),
        );
        headers.append(header::COOKIE, HeaderValue::from_static("b=2"));
        assert_eq!(leer_cookie(&headers, "kiosco_csrf"), Some("xyz"));
        assert_eq!(leer_cookie(&headers, "b"), Some("2"));
        assert_eq!(leer_cookie(&headers, "kiosco"), None);
    }

    #[test]
    fn la_cookie_lleva_secure_solo_si_se_configura() {
        let local = Auth::new(false).cookie("s", "v", Some(60), true);
        assert_eq!(local, "s=v; Path=/; SameSite=Lax; Max-Age=60; HttpOnly");
        let produccion = Auth::new(true).cookie("s", "v", None, false);
        assert_eq!(produccion, "s=v; Path=/; SameSite=Lax; Secure");
    }
}
