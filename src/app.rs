use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rust_embed::RustEmbed;
use tower_http::trace::TraceLayer;

use crate::{
    consumos, dashboard, db, error::AppError, estudiantes, pagos, productos, state::AppState,
};

/// Files under `public/`, embedded into the release binary.
#[derive(RustEmbed)]
#[folder = "public/"]
struct PublicAssets;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(dashboard::handler::index))
        .route(
            "/registro",
            get(consumos::handler::registro).post(consumos::handler::guardar_registro),
        )
        .route("/resumen", get(consumos::handler::resumen))
        .route(
            "/estudiantes",
            get(estudiantes::handler::listar).post(estudiantes::handler::crear),
        )
        .route("/estudiantes/{id}", post(estudiantes::handler::actualizar))
        .route(
            "/estudiantes/{id}/editar",
            get(estudiantes::handler::editar),
        )
        .route(
            "/estudiantes/{id}/estado",
            post(estudiantes::handler::cambiar_estado),
        )
        .route(
            "/productos",
            get(productos::handler::listar).post(productos::handler::crear),
        )
        .route("/productos/{id}", post(productos::handler::actualizar))
        .route("/productos/{id}/editar", get(productos::handler::editar))
        .route(
            "/productos/{id}/estado",
            post(productos::handler::cambiar_estado),
        )
        .route(
            "/pagos/{id_estudiante}",
            get(pagos::handler::cuenta).post(pagos::handler::registrar),
        )
        .route(
            "/pagos/{id_estudiante}/{id_pago}/eliminar",
            post(pagos::handler::eliminar),
        )
        .route("/health", get(health))
        .route("/public/{*path}", get(public_asset))
        .fallback(|| async { AppError::NotFound })
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Result<&'static str, AppError> {
    db::ping(&state.db).await?;
    Ok("ok")
}

async fn public_asset(Path(path): Path<String>) -> Response {
    match PublicAssets::get(&path) {
        Some(file) => {
            let mime = file.metadata.mimetype().to_owned();
            ([(header::CONTENT_TYPE, mime)], file.data).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
