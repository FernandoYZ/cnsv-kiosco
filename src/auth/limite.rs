//! Failed-login limit per client IP: 5 failures in 15 minutes (as in Go).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{extract::ConnectInfo, http::Request};
use std::net::SocketAddr;

const MAXIMO: usize = 5;
const VENTANA: Duration = Duration::from_secs(15 * 60);

#[derive(Clone, Default)]
pub struct LimiteIntentos(Arc<Mutex<HashMap<String, Vec<Instant>>>>);

impl LimiteIntentos {
    pub fn bloqueado(&self, ip: &str) -> bool {
        let mut fallos = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        fallos.get_mut(ip).is_some_and(|lista| {
            lista.retain(|t| t.elapsed() < VENTANA);
            lista.len() >= MAXIMO
        })
    }

    pub fn fallo(&self, ip: &str) {
        let mut fallos = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Forget old entries so the map only holds IPs seen in the last window.
        fallos.retain(|_, lista| {
            lista.retain(|t| t.elapsed() < VENTANA);
            !lista.is_empty()
        });
        fallos
            .entry(ip.to_owned())
            .or_default()
            .push(Instant::now());
    }

    pub fn exito(&self, ip: &str) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(ip);
    }
}

/// Client IP. Behind Caddy it is the LAST `X-Forwarded-For` value, the one
/// the proxy appended; earlier values come from the client and can be forged.
pub fn ip_cliente<B>(request: &Request<B>) -> String {
    let reenviada = request
        .headers()
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .rfind(|v| !v.is_empty());
    match reenviada {
        Some(ip) => ip.to_owned(),
        None => request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map_or_else(|| "desconocida".to_owned(), |c| c.0.ip().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};

    use super::{LimiteIntentos, MAXIMO, ip_cliente};

    #[test]
    fn bloquea_tras_cinco_fallos_y_un_acierto_reinicia() {
        let limite = LimiteIntentos::default();
        for _ in 0..MAXIMO - 1 {
            limite.fallo("1.1.1.1");
        }
        assert!(!limite.bloqueado("1.1.1.1"));
        limite.fallo("1.1.1.1");
        assert!(limite.bloqueado("1.1.1.1"));
        assert!(!limite.bloqueado("2.2.2.2"), "per IP");

        limite.exito("1.1.1.1");
        assert!(!limite.bloqueado("1.1.1.1"));
    }

    #[test]
    fn la_ip_es_la_que_agrego_el_proxy() {
        let request = Request::get("/")
            .header("x-forwarded-for", "6.6.6.6, 10.0.0.1")
            .body(Body::empty())
            .unwrap();
        assert_eq!(ip_cliente(&request), "10.0.0.1");
        let directa = Request::get("/").body(Body::empty()).unwrap();
        assert_eq!(ip_cliente(&directa), "desconocida");
    }
}
