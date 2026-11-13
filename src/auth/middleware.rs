use axum::{
    Extension,
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};

use super::{leer_cookie, sesion};
use crate::state::AppState;

/// Valid session cookie → `Extension<Sesion>`; otherwise clear it and go to `/login`.
pub async fn requiere_sesion(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = leer_cookie(request.headers(), sesion::COOKIE);
    let ahora = chrono::Utc::now().timestamp();
    if let Some(sesion) = token.and_then(|t| state.auth.llave.verificar(t, ahora)) {
        request.extensions_mut().insert(sesion);
        return next.run(request).await;
    }
    let mut respuesta = Redirect::to("/login").into_response();
    if token.is_some() {
        let borrar = state.auth.cookie(sesion::COOKIE, "", Some(0), true);
        respuesta.headers_mut().append(header::SET_COOKIE, borrar);
    }
    respuesta
}

/// Users without `puede_editar` are sent to the consumption screen (as in Go).
/// Must run after [`requiere_sesion`].
pub async fn requiere_edicion(
    Extension(sesion): Extension<sesion::Sesion>,
    request: Request,
    next: Next,
) -> Response {
    if sesion.puede_editar {
        next.run(request).await
    } else {
        tracing::info!(id_usuario = sesion.id_usuario, path = %request.uri().path(), "edit permission required");
        Redirect::to("/registro").into_response()
    }
}
