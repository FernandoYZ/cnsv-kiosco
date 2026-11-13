use std::path::PathBuf;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Errors that prevent the server from starting or stopping cleanly.
#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("cannot read config file {path}: {source}")]
    ConfigRead {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid config file {path}: {source}")]
    ConfigParse {
        path: String,
        source: toml::de::Error,
    },
    #[error("invalid value for {name}: {value}")]
    InvalidEnv { name: &'static str, value: String },
    #[error("database file not found: {} (it is never created automatically)", .0.display())]
    DatabaseMissing(PathBuf),
    #[error("cannot open database: {0}")]
    DatabaseOpen(#[from] tokio_rusqlite::rusqlite::Error),
    #[error("database error: {0}")]
    Database(#[from] tokio_rusqlite::Error),
    #[error("server I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors raised while handling an HTTP request.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] tokio_rusqlite::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Database(err) => {
                tracing::error!(error = %err, "database failure");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error interno del servidor",
                )
                    .into_response()
            }
        }
    }
}
