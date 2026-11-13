use axum::{
    Form,
    extract::{Path, Query, State},
    response::Redirect,
};
use maud::{Markup, html};
use serde::Deserialize;

use super::{Estudiante, GRADOS, grado_desde, grado_valido, repository};
use crate::{
    error::AppError,
    forms,
    state::AppState,
    templates::{
        components::{Icono, avatar, bottom_nav, encabezado, estado_vacio, ios_nav, mensaje_flash},
        layout,
    },
};

#[derive(Deserialize)]
pub struct ListaQuery {
    grado: Option<String>,
    editar: Option<i64>,
    ok: Option<String>,
}

#[derive(Deserialize)]
pub struct EstudianteForm {
    nombres: String,
    apellidos: String,
    id_grado: String,
    /// Grade filter to return to after saving.
    filtro: Option<String>,
}

#[derive(Deserialize)]
pub struct EstadoForm {
    activo: u8,
    filtro: Option<String>,
}

struct DatosEstudiante {
    nombres: String,
    apellidos: String,
    id_grado: i64,
}

fn validar(form: &EstudianteForm) -> Result<DatosEstudiante, AppError> {
    let id_grado = form
        .id_grado
        .trim()
        .parse()
        .ok()
        .filter(|id| grado_valido(*id))
        .ok_or_else(|| AppError::BadRequest("Selecciona un grado válido.".to_owned()))?;
    Ok(DatosEstudiante {
        nombres: forms::texto(&form.nombres, "Nombres", 100)?,
        apellidos: forms::texto(&form.apellidos, "Apellidos", 100)?,
        id_grado,
    })
}

fn url_lista(filtro: Option<i64>, ok: &str) -> String {
    match filtro {
        Some(grado) => format!("/estudiantes?grado={grado}&ok={ok}"),
        None => format!("/estudiantes?ok={ok}"),
    }
}

pub async fn listar(
    State(state): State<AppState>,
    Query(q): Query<ListaQuery>,
) -> Result<Markup, AppError> {
    let filtro = grado_desde(q.grado.as_deref());
    let estudiantes = state.db.call(|c| repository::todos(c)).await?;
    let visibles: Vec<&Estudiante> = estudiantes
        .iter()
        .filter(|e| filtro.is_none() || e.id_grado == filtro)
        .collect();
    Ok(vista(
        &visibles,
        filtro,
        q.editar,
        mensaje_flash(q.ok.as_deref()),
    ))
}

pub async fn crear(
    State(state): State<AppState>,
    Form(form): Form<EstudianteForm>,
) -> Result<Redirect, AppError> {
    let datos = validar(&form)?;
    state
        .db
        .call(move |c| repository::insertar(c, &datos.nombres, &datos.apellidos, datos.id_grado))
        .await?;
    Ok(Redirect::to(&url_lista(
        grado_desde(form.filtro.as_deref()),
        "estudiante-creado",
    )))
}

pub async fn actualizar(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<EstudianteForm>,
) -> Result<Redirect, AppError> {
    let datos = validar(&form)?;
    let filas = state
        .db
        .call(move |c| {
            repository::actualizar(c, id, &datos.nombres, &datos.apellidos, datos.id_grado)
        })
        .await?;
    if filas == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Redirect::to(&url_lista(
        grado_desde(form.filtro.as_deref()),
        "estudiante-actualizado",
    )))
}

pub async fn cambiar_estado(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<EstadoForm>,
) -> Result<Redirect, AppError> {
    let activo = form.activo == 1;
    let filas = state
        .db
        .call(move |c| repository::cambiar_estado(c, id, activo))
        .await?;
    if filas == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Redirect::to(&url_lista(
        grado_desde(form.filtro.as_deref()),
        "estudiante-estado",
    )))
}

fn opciones_grado(seleccionado: Option<i64>) -> Markup {
    html! {
        @for g in &GRADOS {
            option value=(g.id) selected[seleccionado == Some(g.id)] { (g.nombre) }
        }
    }
}

fn campo_filtro(filtro: Option<i64>) -> Markup {
    html! {
        @if let Some(grado) = filtro { input type="hidden" name="filtro" value=(grado); }
    }
}

fn fila(e: &Estudiante, filtro: Option<i64>) -> Markup {
    let editar = match filtro {
        Some(g) => format!("/estudiantes?grado={g}&editar={}#e{}", e.id, e.id),
        None => format!("/estudiantes?editar={}#e{}", e.id, e.id),
    };
    html! {
        div class="flex items-center justify-between gap-2 p-4 transition-colors hover:bg-gray-50" {
            div class="flex min-w-0 items-center gap-4" {
                (avatar(&e.apellidos, e.esta_activo))
                div class="min-w-0" {
                    p class=(if e.esta_activo { "truncate text-[17px] leading-tight font-semibold text-gray-900" } else { "truncate text-[17px] leading-tight font-semibold text-gray-400 line-through" }) {
                        (e.nombre_completo())
                    }
                    p class="text-[15px] font-medium text-ios-gray" { (e.grado()) }
                }
            }
            div class="flex shrink-0 items-center gap-1" {
                a href=(editar) class="btn-text text-ios-blue hover:bg-blue-50" { "Editar" }
                form method="post" action=(format!("/estudiantes/{}/estado", e.id))
                    data-confirm=[e.esta_activo.then(|| format!("¿Desactivar a {}?", e.nombres))] {
                    input type="hidden" name="activo" value=(if e.esta_activo { "0" } else { "1" });
                    (campo_filtro(filtro))
                    @if e.esta_activo {
                        button type="submit" class="btn-text text-ios-red hover:bg-red-50" { "Bajar" }
                    } @else {
                        button type="submit" class="btn-text text-ios-green hover:bg-green-50" { "Activar" }
                    }
                }
            }
        }
    }
}

fn fila_edicion(e: &Estudiante, filtro: Option<i64>) -> Markup {
    let cancelar = match filtro {
        Some(g) => format!("/estudiantes?grado={g}"),
        None => "/estudiantes".to_owned(),
    };
    html! {
        div class="border-l-4 border-ios-blue bg-[#f9f9f9] p-5" {
            form method="post" action=(format!("/estudiantes/{}", e.id)) class="space-y-4" {
                (campo_filtro(filtro))
                div class="grid grid-cols-1 gap-4 sm:grid-cols-2" {
                    label class="block rounded-xl border border-gray-200 bg-white p-3 shadow-sm" {
                        span class="mb-1 block text-[12px] font-bold text-gray-400 uppercase" { "Apellidos" }
                        input type="text" name="apellidos" value=(e.apellidos) required maxlength="100" autofocus
                            class="w-full border-none bg-transparent p-0 text-[17px] font-medium text-gray-900";
                    }
                    label class="block rounded-xl border border-gray-200 bg-white p-3 shadow-sm" {
                        span class="mb-1 block text-[12px] font-bold text-gray-400 uppercase" { "Nombres" }
                        input type="text" name="nombres" value=(e.nombres) required maxlength="100"
                            class="w-full border-none bg-transparent p-0 text-[17px] font-medium text-gray-900";
                    }
                }
                label class="block rounded-xl border border-gray-200 bg-white p-3 shadow-sm" {
                    span class="mb-1 block text-[12px] font-bold text-gray-400 uppercase" { "Grado" }
                    select name="id_grado" required class="w-full appearance-none border-none bg-transparent p-0 text-[17px] font-medium text-gray-900" {
                        (opciones_grado(e.id_grado))
                    }
                }
                div class="flex gap-3" {
                    button type="submit" class="flex-1 rounded-xl bg-ios-blue py-3 font-bold text-white shadow-md transition-all active:scale-95" { "Guardar Cambios" }
                    a href=(cancelar) class="rounded-xl border border-gray-200 bg-white px-6 py-3 font-semibold text-gray-600 transition-all active:scale-95" { "Cancelar" }
                }
            }
        }
    }
}

fn vista(
    estudiantes: &[&Estudiante],
    filtro: Option<i64>,
    editar: Option<i64>,
    flash: Option<&str>,
) -> Markup {
    let pill = |href: String, etiqueta: &str, activo: bool| {
        html! { a href=(href) class=(if activo { "pill pill-active" } else { "pill" }) { (etiqueta) } }
    };
    layout::page(
        "Estudiantes",
        flash,
        html! {
            (bottom_nav())
            (ios_nav(("/", "Atrás"), "Configuración", html! {}))
            main class="mx-auto max-w-2xl px-4 pt-8 pb-32 sm:pb-10 lg:max-w-6xl" {
                (encabezado("Estudiantes", "Gestión de alumnos por grado"))
                div class="items-start lg:grid lg:grid-cols-12 lg:gap-10" {
                    aside class="mb-10 lg:sticky lg:top-24 lg:col-span-5" {
                        h3 class="section-title" { "Nuevo registro" }
                        div class="ios-card rounded-[32px]" {
                            form method="post" action="/estudiantes" class="divide-y divide-gray-100" {
                                (campo_filtro(filtro))
                                div class="ios-row" {
                                    label for="new-last" class="ios-label w-20" { "Apellidos" }
                                    input id="new-last" type="text" name="apellidos" placeholder="Ej. Bazán" required maxlength="100" class="ios-input";
                                }
                                div class="ios-row" {
                                    label for="new-first" class="ios-label w-20" { "Nombres" }
                                    input id="new-first" type="text" name="nombres" placeholder="Ej. Fernando" required maxlength="100" class="ios-input";
                                }
                                div class="ios-row" {
                                    label for="new-grade" class="ios-label w-20" { "Grado" }
                                    select id="new-grade" name="id_grado" required class="ios-input appearance-none font-bold text-ios-blue" {
                                        (opciones_grado(filtro.or(Some(GRADOS[0].id))))
                                    }
                                }
                                div class="bg-gray-50/50 p-4" {
                                    button type="submit" class="btn-ios" { "Registrar Estudiante" }
                                }
                            }
                        }
                    }
                    section class="lg:col-span-7" {
                        div class="mb-3 flex items-center justify-between px-4" {
                            h3 class="text-[13px] font-bold tracking-wide text-ios-gray uppercase" { "Listado general" }
                            span class="badge bg-indigo-50 text-indigo-600" { (estudiantes.len()) " total" }
                        }
                        nav class="no-scrollbar mb-6 flex gap-2 overflow-x-auto px-1 pb-1" aria-label="Filtrar por grado" {
                            (pill("/estudiantes".to_owned(), "Todos", filtro.is_none()))
                            @for g in &GRADOS {
                                (pill(format!("/estudiantes?grado={}", g.id), g.nombre, filtro == Some(g.id)))
                            }
                        }
                        div class="ios-card rounded-[32px]" {
                            @if estudiantes.is_empty() {
                                (estado_vacio(Icono::Estudiantes, "Sin alumnos", "Comienza agregando estudiantes en el panel lateral."))
                            } @else {
                                div class="divide-y divide-gray-100" {
                                    @for e in estudiantes {
                                        div id=(format!("e{}", e.id)) class="scroll-mt-24" {
                                            @if editar == Some(e.id) { (fila_edicion(e, filtro)) } @else { (fila(e, filtro)) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}
