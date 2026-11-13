use axum::{
    Form,
    extract::{Path, State},
    response::Redirect,
};
use maud::{Markup, html};
use serde::Deserialize;

use super::{Estudiante, GRADOS, grado_valido, repository};
use crate::{
    error::AppError,
    forms,
    state::AppState,
    templates::{
        components::{badge_estado, boton_estado, page_header},
        layout,
    },
};

#[derive(Deserialize)]
pub struct EstudianteForm {
    nombres: String,
    apellidos: String,
    id_grado: String,
}

#[derive(Deserialize)]
pub struct EstadoForm {
    activo: u8,
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

pub async fn listar(State(state): State<AppState>) -> Result<Markup, AppError> {
    let estudiantes = state.db.call(|c| repository::todos(c)).await?;
    Ok(vista_lista(&estudiantes))
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
    Ok(Redirect::to("/estudiantes"))
}

pub async fn editar(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Markup, AppError> {
    let estudiante = state
        .db
        .call(move |c| repository::por_id(c, id))
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(vista_editar(&estudiante))
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
    Ok(Redirect::to("/estudiantes"))
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
    Ok(Redirect::to("/estudiantes"))
}

/// Grade `<select>`. With `opcion_todos`, value `0` means every grade.
pub fn select_grado(name: &str, seleccionado: Option<i64>, opcion_todos: bool) -> Markup {
    html! {
        select class="input" id=(name) name=(name) required[!opcion_todos] {
            @if opcion_todos {
                option value="0" selected[seleccionado.unwrap_or(0) == 0] { "Todos los grados" }
            } @else {
                option value="" disabled selected[seleccionado.is_none()] { "Selecciona…" }
            }
            @for g in &GRADOS {
                option value=(g.id) selected[seleccionado == Some(g.id)] { (g.nombre) }
            }
        }
    }
}

fn campos(estudiante: Option<&Estudiante>) -> Markup {
    html! {
        div {
            label class="label" for="apellidos" { "Apellidos" }
            input class="input w-56" id="apellidos" name="apellidos" required maxlength="100"
                value=[estudiante.map(|e| e.apellidos.as_str())];
        }
        div {
            label class="label" for="nombres" { "Nombres" }
            input class="input w-56" id="nombres" name="nombres" required maxlength="100"
                value=[estudiante.map(|e| e.nombres.as_str())];
        }
        div {
            label class="label" for="id_grado" { "Grado" }
            (select_grado("id_grado", estudiante.and_then(|e| e.id_grado), false))
        }
    }
}

fn vista_lista(estudiantes: &[Estudiante]) -> Markup {
    let activos = estudiantes.iter().filter(|e| e.esta_activo).count();
    layout::page(
        "Estudiantes",
        "estudiantes",
        html! {
            (page_header("Estudiantes", Some(&format!("{activos} activos de {} registrados", estudiantes.len()))))
            form method="post" action="/estudiantes" class="card mb-6 flex flex-wrap items-end gap-4" {
                (campos(None))
                button type="submit" class="btn-primary" { "Agregar estudiante" }
            }
            div class="card overflow-x-auto p-0" {
                table class="table-base" {
                    thead { tr { th { "Estudiante" } th { "Grado" } th { "Estado" } th {} } }
                    tbody {
                        @for e in estudiantes {
                            tr class=(if e.esta_activo { "" } else { "text-slate-400" }) {
                                td { (e.nombre_completo()) }
                                td class="whitespace-nowrap" { (e.grado()) }
                                td { (badge_estado(e.esta_activo)) }
                                td class="space-x-3 text-right whitespace-nowrap" {
                                    a class="btn-link" href=(format!("/estudiantes/{}/editar", e.id)) { "Editar" }
                                    (boton_estado(&format!("/estudiantes/{}/estado", e.id), e.esta_activo))
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

fn vista_editar(estudiante: &Estudiante) -> Markup {
    layout::page(
        "Editar estudiante",
        "estudiantes",
        html! {
            (page_header("Editar estudiante", Some(&estudiante.nombre_completo())))
            form method="post" action=(format!("/estudiantes/{}", estudiante.id)) class="card flex flex-wrap items-end gap-4" {
                (campos(Some(estudiante)))
                button type="submit" class="btn-primary" { "Guardar" }
                a href="/estudiantes" class="btn-secondary" { "Cancelar" }
            }
        },
    )
}
