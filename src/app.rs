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
        // Consumption
        .route("/registro", get(consumos::handler::inicio))
        .route("/registro/{sector}", get(consumos::handler::sector))
        .route("/resumen/{sector}", get(consumos::handler::resumen))
        .route(
            "/consumos/{id_estudiante}",
            get(consumos::handler::editor).post(consumos::handler::guardar),
        )
        // Payments
        .route(
            "/pagos/{id_estudiante}",
            get(pagos::handler::gestion).post(pagos::handler::registrar),
        )
        .route(
            "/pagos/{id_estudiante}/{id_pago}/eliminar",
            post(pagos::handler::eliminar),
        )
        .route(
            "/comprobante/{id_estudiante}",
            get(pagos::handler::comprobante),
        )
        // Configuration
        .route(
            "/estudiantes",
            get(estudiantes::handler::listar).post(estudiantes::handler::crear),
        )
        .route("/estudiantes/{id}", post(estudiantes::handler::actualizar))
        .route(
            "/estudiantes/{id}/estado",
            post(estudiantes::handler::cambiar_estado),
        )
        .route(
            "/productos",
            get(productos::handler::listar).post(productos::handler::crear),
        )
        .route("/productos/{id}", post(productos::handler::actualizar))
        .route(
            "/productos/{id}/estado",
            post(productos::handler::cambiar_estado),
        )
        // Infrastructure
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
