use tokio_rusqlite::Connection;

use crate::{auth::Auth, pagos::envios::EnviosRecientes};

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub db: Connection,
    /// One-time tokens of the payment form (prevents double payments).
    pub envios_pago: EnviosRecientes,
    pub auth: Auth,
}
