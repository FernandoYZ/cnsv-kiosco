use axum::{
    Extension, Form,
    extract::{FromRequest, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use maud::{Markup, html};
use serde::Deserialize;

use super::{
    csrf::{CAMPO, TokenCsrf},
    leer_cookie, limite, password, repository,
    sesion::{self, Sesion},
};
use crate::{
    error::AppError,
    state::AppState,
    templates::{
        components::{Icono, icono},
        layout,
    },
};

const CREDENCIALES: &str = "Usuario o contraseña incorrectos";

/// Error box of the login page: title (optional) and message, Go texts verbatim.
fn aviso(codigo: &str) -> Option<(Option<&'static str>, &'static str)> {
    Some(match codigo {
        "credenciales" => (None, CREDENCIALES),
        "rate_limit" => (
            Some("Demasiados intentos"),
            "Has realizado demasiados intentos de login. Intenta de nuevo en 15 minutos.",
        ),
        "csrf" => (
            Some("Token de seguridad expirado"),
            "El token de seguridad ha expirado. Recarga la página e intenta de nuevo.",
        ),
        _ => return None,
    })
}

#[derive(Deserialize)]
pub struct LoginQuery {
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginForm {
    usuario: String,
    password: String,
}

/// `GET /login`. Already logged in → `/`.
pub async fn mostrar(
    State(state): State<AppState>,
    Extension(token): Extension<TokenCsrf>,
    headers: HeaderMap,
    Query(q): Query<LoginQuery>,
) -> Response {
    let ahora = chrono::Utc::now().timestamp();
    let con_sesion = leer_cookie(&headers, sesion::COOKIE)
        .and_then(|t| state.auth.llave.verificar(t, ahora))
        .is_some();
    if con_sesion {
        return Redirect::to("/").into_response();
    }
    vista(&token.0, q.error.as_deref().and_then(aviso), "").into_response()
}

/// `POST /login`. The CSRF middleware already checked the token.
pub async fn ingresar(
    State(state): State<AppState>,
    Extension(token): Extension<TokenCsrf>,
    request: Request,
) -> Result<Response, AppError> {
    let ip = limite::ip_cliente(&request);
    if state.auth.intentos.bloqueado(&ip) {
        tracing::warn!(%ip, "login blocked by rate limit");
        let pagina = vista(&token.0, aviso("rate_limit"), "");
        return Ok((StatusCode::TOO_MANY_REQUESTS, pagina).into_response());
    }
    let Form(form) = Form::<LoginForm>::from_request(request, &state)
        .await
        .map_err(|_| AppError::BadRequest("Formulario no válido.".to_owned()))?;

    let usuario = form.usuario.trim().to_owned();
    let nombre = usuario.clone();
    let clave = form.password;
    let sesion = state
        .db
        .call(move |c| {
            let encontrado = repository::por_nombre(c, &nombre)?;
            Ok::<_, tokio_rusqlite::rusqlite::Error>(match encontrado {
                Some(u) if password::verificar(&clave, &u.contrasenha) => Some(Sesion {
                    id_usuario: u.id,
                    puede_editar: u.puede_editar,
                }),
                Some(_) => None,
                None => {
                    // Same work as a wrong password, so timing does not reveal users.
                    password::verificar(&clave, &password::HASH_SEÑUELO);
                    None
                }
            })
        })
        .await?;

    let Some(sesion) = sesion else {
        state.auth.intentos.fallo(&ip);
        tracing::warn!(%ip, "failed login");
        let pagina = vista(&token.0, aviso("credenciales"), &usuario);
        return Ok((StatusCode::UNAUTHORIZED, pagina).into_response());
    };

    state.auth.intentos.exito(&ip);
    tracing::info!(id_usuario = sesion.id_usuario, "login");
    let valor = state
        .auth
        .llave
        .firmar(sesion, chrono::Utc::now().timestamp());
    let cookie = state.auth.cookie(
        sesion::COOKIE,
        &valor,
        Some(sesion::DURACION_SEGUNDOS),
        true,
    );
    let mut respuesta = Redirect::to("/").into_response();
    respuesta.headers_mut().append(header::SET_COOKIE, cookie);
    Ok(respuesta)
}

/// `GET /logout` (as in Go).
pub async fn salir(State(state): State<AppState>) -> Response {
    let borrar = state.auth.cookie(sesion::COOKIE, "", Some(0), true);
    let mut respuesta = Redirect::to("/login").into_response();
    respuesta.headers_mut().append(header::SET_COOKIE, borrar);
    respuesta
}

fn vista(token: &str, error: Option<(Option<&str>, &str)>, usuario: &str) -> Markup {
    layout::page(
        "Iniciar Sesión",
        None,
        html! {
            main class="flex min-h-screen items-center justify-center px-4 py-10" {
                div class="w-full max-w-sm" {
                    div class="mb-8 text-center" {
                        div class="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-[18px] bg-ios-blue text-white shadow-lg" {
                            (icono(Icono::Productos, "h-9 w-9"))
                        }
                        h1 class="text-[28px] leading-tight font-bold tracking-tight text-gray-900" {
                            "Kiosco " span class="text-ios-blue" { "del Valle" }
                        }
                        p class="mt-1 text-[15px] text-ios-gray" { "Identifícate para continuar" }
                    }
                    @if let Some((titulo, mensaje)) = error {
                        div role="alert" class="mb-4 flex items-start gap-3 rounded-2xl border border-red-100 bg-red-50 p-4 text-ios-red" {
                            span class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-ios-red text-[14px] font-bold text-white" { "!" }
                            div class="text-[14px]" {
                                @if let Some(titulo) = titulo { p class="font-bold" { (titulo) } }
                                p class="font-medium" { (mensaje) }
                            }
                        }
                    }
                    form method="post" action="/login" {
                        input type="hidden" name=(CAMPO) value=(token);
                        div class="ios-card divide-y divide-gray-100" {
                            div class="ios-row" {
                                label for="usuario" class="ios-label w-20" { "Usuario" }
                                input id="usuario" type="text" name="usuario" value=(usuario) placeholder="Requerido"
                                    required autofocus autocomplete="username" autocapitalize="none" spellcheck="false" class="ios-input";
                            }
                            div class="ios-row" {
                                label for="password" class="ios-label w-20" { "Clave" }
                                input id="password" type="password" name="password" placeholder="••••••••"
                                    required autocomplete="current-password" class="ios-input";
                            }
                        }
                        button type="submit" class="btn-ios mt-6" { "Iniciar Sesión" }
                    }
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::aviso;

    #[test]
    fn los_mensajes_son_los_de_la_app_go() {
        assert_eq!(
            aviso("credenciales"),
            Some((None, "Usuario o contraseña incorrectos"))
        );
        assert_eq!(aviso("rate_limit").unwrap().0, Some("Demasiados intentos"));
        assert_eq!(
            aviso("csrf").unwrap().0,
            Some("Token de seguridad expirado")
        );
        assert_eq!(aviso("otro"), None);
    }
}
