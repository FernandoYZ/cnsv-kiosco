//! Static files under `public/`, embedded into the release binary.
//!
//! Pages reference assets through [`url`], which appends a content hash
//! (`/public/css/app.css?v=1a2b3c4d`). Versioned requests are cached by the
//! browser for a year; when the file changes the URL changes too. Every
//! response carries an `ETag`, so unversioned requests (fonts referenced from
//! the CSS) are revalidated with a cheap `304 Not Modified`.

use axum::{
    extract::{Path, RawQuery},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "public/"]
struct PublicAssets;

const CACHE_VERSIONADO: &str = "public, max-age=31536000, immutable";
const CACHE_SIN_VERSION: &str = "public, max-age=3600";

fn hash_corto(hash: &[u8; 32]) -> String {
    hash[..4].iter().map(|b| format!("{b:02x}")).collect()
}

/// Versioned URL of a file inside `public/` (e.g. `css/app.css`).
pub fn url(ruta: &str) -> String {
    match PublicAssets::get(ruta) {
        Some(archivo) => format!(
            "/public/{ruta}?v={}",
            hash_corto(&archivo.metadata.sha256_hash())
        ),
        None => format!("/public/{ruta}"),
    }
}

/// `GET /public/{*ruta}`
pub async fn servir(
    Path(ruta): Path<String>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> Response {
    let Some(archivo) = PublicAssets::get(&ruta) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let etag = format!("\"{}\"", hash_corto(&archivo.metadata.sha256_hash()));
    let versionado = query.is_some_and(|q| q.starts_with("v="));
    let cache = if versionado {
        CACHE_VERSIONADO
    } else {
        CACHE_SIN_VERSION
    };

    let sin_cambios = headers
        .get(header::IF_NONE_MATCH)
        .is_some_and(|valor| valor.as_bytes() == etag.as_bytes());
    if sin_cambios {
        return (
            StatusCode::NOT_MODIFIED,
            [
                (header::ETAG, etag),
                (header::CACHE_CONTROL, cache.to_owned()),
            ],
        )
            .into_response();
    }

    (
        [
            (header::CONTENT_TYPE, archivo.metadata.mimetype().to_owned()),
            (header::ETAG, etag),
            (header::CACHE_CONTROL, cache.to_owned()),
        ],
        archivo.data,
    )
        .into_response()
}
