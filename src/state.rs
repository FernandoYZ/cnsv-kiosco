use tokio_rusqlite::Connection;

/// Shared application state. Sessions will be added with the `auth` module.
#[derive(Clone)]
pub struct AppState {
    pub db: Connection,
}
