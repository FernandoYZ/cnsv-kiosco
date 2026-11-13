//! One-time tokens for the payment form.
//!
//! Registering a payment is not idempotent (it is an INSERT). Each rendered
//! form carries a fresh token; the first submission consumes it and any later
//! submission with the same token (double tap, browser back + resubmit) is
//! recognised as a duplicate and not written again.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// How long a consumed token is remembered.
const VIGENCIA: Duration = Duration::from_secs(60 * 60);

#[derive(Clone, Default)]
pub struct EnviosRecientes {
    usados: Arc<Mutex<HashMap<String, Instant>>>,
    contador: Arc<AtomicU64>,
}

impl EnviosRecientes {
    /// Unique token for a new form. Uniqueness, not secrecy, is what matters here.
    pub fn nuevo_token(&self) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let n = self.contador.fetch_add(1, Ordering::Relaxed);
        format!("{nanos:x}-{n:x}")
    }

    /// Marks `token` as consumed. Returns `false` when it was already consumed.
    pub fn consumir(&self, token: &str) -> bool {
        let mut usados = self.usados.lock().unwrap_or_else(|e| e.into_inner());
        let ahora = Instant::now();
        usados.retain(|_, usado| ahora.duration_since(*usado) < VIGENCIA);
        if usados.contains_key(token) {
            return false;
        }
        usados.insert(token.to_owned(), ahora);
        true
    }

    /// Releases a token whose payment could not be saved, so it can be retried.
    pub fn liberar(&self, token: &str) {
        self.usados
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(token);
    }
}

#[cfg(test)]
mod tests {
    use super::EnviosRecientes;

    #[test]
    fn un_token_solo_se_puede_consumir_una_vez() {
        let envios = EnviosRecientes::default();
        let token = envios.nuevo_token();
        assert!(envios.consumir(&token));
        assert!(!envios.consumir(&token));
    }

    #[test]
    fn liberar_un_token_permite_reintentar() {
        let envios = EnviosRecientes::default();
        let token = envios.nuevo_token();
        assert!(envios.consumir(&token));
        envios.liberar(&token);
        assert!(envios.consumir(&token));
    }

    #[test]
    fn cada_formulario_recibe_un_token_distinto() {
        let envios = EnviosRecientes::default();
        assert_ne!(envios.nuevo_token(), envios.nuevo_token());
    }
}
