use axum::{
    Router,
    extract::State,
    http::{HeaderValue, header},
    routing::{get, post},
};
use tower_http::{
    compression::{
        CompressionLayer,
        predicate::{DefaultPredicate, NotForContentType, Predicate},
    },
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

use crate::{
    assets, consumos, dashboard, db, error::AppError, estudiantes, pagos, productos,
    state::AppState,
};

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
        .route("/health", get(health))
        .fallback(|| async { AppError::NotFound })
        // Dynamic pages must never come from the browser cache: going back
        // after a POST would show stale data and invite a duplicate submit.
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        // Added after the layer above, so static assets stay cacheable.
        .route("/public/{*ruta}", get(assets::servir))
        // HTML and CSS shrink ~10x with gzip; fonts and images are already compressed.
        .layer(
            CompressionLayer::new()
                .gzip(true)
                .compress_when(DefaultPredicate::new().and(NotForContentType::const_new("font/"))),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Result<&'static str, AppError> {
    db::ping(&state.db).await?;
    Ok("ok")
}
