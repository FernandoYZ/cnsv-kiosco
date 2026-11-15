use axum::{
    Router,
    extract::State,
    http::{HeaderValue, header},
    middleware,
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
    assets, auth, cierre, cobranza, consumos, dashboard, db, error::AppError, estudiantes, pagos,
    productos, state::AppState,
};

pub fn router(state: AppState) -> Router {
    // Any logged-in user (also `puede_editar = 0`): register consumption, as in Go.
    let con_sesion = Router::new()
        .route("/registro", get(consumos::handler::inicio))
        .route("/registro/{sector}", get(consumos::handler::sector))
        .route("/resumen/{sector}", get(consumos::handler::resumen))
        .route(
            "/consumos/{id_estudiante}",
            get(consumos::handler::editor).post(consumos::handler::guardar),
        )
        .route("/productos", get(productos::handler::listar));

    // Everything else needs `puede_editar = 1`.
    let con_edicion = Router::new()
        .route("/", get(dashboard::handler::index))
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
        // Collections and day close
        .route("/cobranza", get(cobranza::handler::index))
        .route("/cierre", get(cierre::handler::index))
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
        .route("/productos", post(productos::handler::crear))
        .route("/productos/{id}", post(productos::handler::actualizar))
        .route(
            "/productos/{id}/estado",
            post(productos::handler::cambiar_estado),
        )
        .route_layer(middleware::from_fn(auth::middleware::requiere_edicion));

    Router::new()
        .merge(
            con_sesion
                .merge(con_edicion)
                .route_layer(middleware::from_fn_with_state(
                    state.clone(),
                    auth::middleware::requiere_sesion,
                )),
        )
        .route(
            "/login",
            get(auth::handler::mostrar).post(auth::handler::ingresar),
        )
        .route("/logout", post(auth::handler::salir))
        .route("/health", get(health))
        .fallback(|| async { AppError::NotFound })
        // Every dynamic route: CSRF cookie and POST token check.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::csrf::proteger,
        ))
        // Dynamic pages must never come from the browser cache: going back
        // after a POST would show stale data and invite a duplicate submit.
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        // Added after the layer above, so static assets stay cacheable.
        .route("/public/{*ruta}", get(assets::servir))
        .route("/sw.js", get(assets::service_worker))
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

#[cfg(test)]
mod tests;
