use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use maud::{Markup, html};
use rust_embed::RustEmbed;
use tower_http::trace::TraceLayer;

use crate::{
    db,
    error::AppError,
    state::AppState,
    templates::{components, layout},
};

/// Files under `public/`, embedded into the release binary.
#[derive(RustEmbed)]
#[folder = "public/"]
struct PublicAssets;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(home))
        .route("/health", get(health))
        .route("/public/{*path}", get(public_asset))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Temporary page until the `dashboard` module exists.
async fn home() -> Markup {
    layout::page(
        "Inicio",
        html! {
            (components::page_header("Kiosco CNSV"))
            p { "Esqueleto del proyecto en funcionamiento." }
        },
    )
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
