use std::path::PathBuf;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use maud::html;

use crate::templates::{
    components::{Icono, icono},
    layout,
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
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found")]
    NotFound,
    /// POST without the CSRF token of the browser session.
    #[error("invalid CSRF token")]
    CsrfInvalido,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, titulo, mensaje) = match self {
            AppError::Database(err) => {
                tracing::error!(error = %err, "database failure");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error del servidor",
                    "Ocurrió un error interno. Intenta nuevamente.".to_owned(),
                )
            }
            AppError::BadRequest(mensaje) => (StatusCode::BAD_REQUEST, "Datos no válidos", mensaje),
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                "Página no encontrada",
                "El recurso que buscas no existe o fue movido.".to_owned(),
            ),
            AppError::CsrfInvalido => (
                StatusCode::FORBIDDEN,
                "Token de seguridad expirado",
                "El token de seguridad ha expirado. Recarga la página e intenta de nuevo."
                    .to_owned(),
            ),
        };

        let pagina = layout::page(
            titulo,
            None,
            html! {
                main class="flex min-h-screen flex-col px-6 pt-20 sm:items-center sm:justify-center sm:pt-0" {
                    div class="mx-auto w-full max-w-[360px]" {
                        div class="mb-10 flex flex-col items-center" {
                            div class="mb-6 flex h-20 w-20 items-center justify-center rounded-[22%] bg-red-50 shadow-xl shadow-red-100" {
                                (icono(Icono::Alerta, "h-10 w-10 text-red-500"))
                            }
                            h1 class="text-center text-[28px] leading-tight font-extrabold tracking-tight text-gray-900" { (titulo) }
                        }
                        div class="mb-8 rounded-2xl border border-red-200 bg-white/50 p-4" {
                            p class="text-[15px] leading-relaxed font-semibold text-red-600" { (mensaje) }
                        }
                        div class="space-y-3" {
                            a href="/" class="btn-ios" { "Ir al inicio" }
                            button type="button" data-back class="btn-ios-secondary" { "Volver" }
                        }
                        p class="mt-10 text-center text-[13px] leading-relaxed font-medium text-ios-gray" {
                            "Sistema de Gestión Interna" br; "Kiosco del Valle"
                        }
                    }
                }
            },
        );
        (status, pagina).into_response()
    }
}
