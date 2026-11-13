use tokio_rusqlite::Connection;

use crate::pagos::envios::EnviosRecientes;

/// Shared application state. Sessions will be added with the `auth` module.
#[derive(Clone)]
pub struct AppState {
    pub db: Connection,
    /// One-time tokens of the payment form (prevents double payments).
    pub envios_pago: EnviosRecientes,
}
