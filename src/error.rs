use std::path::PathBuf;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use maud::html;

use crate::templates::layout;

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
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found")]
    NotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, mensaje) = match self {
            AppError::Database(err) => {
                tracing::error!(error = %err, "database failure");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Ocurrió un error interno. Intenta nuevamente.".to_owned(),
                )
            }
            AppError::BadRequest(mensaje) => (StatusCode::BAD_REQUEST, mensaje),
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                "El recurso solicitado no existe.".to_owned(),
            ),
        };

        let pagina = layout::page(
            "Error",
            "",
            html! {
                div class="card mx-auto max-w-lg" {
                    h1 class="mb-2 text-lg font-semibold text-red-700" { "No se pudo completar la acción" }
                    p class="mb-4 text-slate-700" { (mensaje) }
                    button type="button" class="btn-secondary" onclick="history.back()" { "Volver" }
                }
            },
        );
        (status, pagina).into_response()
    }
}
