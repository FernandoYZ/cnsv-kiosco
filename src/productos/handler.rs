use axum::{
    Form,
    extract::{Path, State},
    response::Redirect,
};
use maud::{Markup, html};
use serde::Deserialize;

use super::{Producto, repository};
use crate::{
    error::AppError,
    forms,
    state::AppState,
    templates::{
        components::{badge_estado, boton_estado, page_header, soles},
        layout,
    },
};

#[derive(Deserialize)]
pub struct ProductoForm {
    nombre: String,
    precio: String,
}

#[derive(Deserialize)]
pub struct EstadoForm {
    activo: u8,
}

fn validar(form: &ProductoForm) -> Result<(String, f64), AppError> {
    Ok((
        forms::texto(&form.nombre, "Nombre", 100)?,
        forms::monto(&form.precio, "Precio")?,
    ))
}

pub async fn listar(State(state): State<AppState>) -> Result<Markup, AppError> {
    let productos = state.db.call(|c| repository::todos(c)).await?;
    Ok(vista_lista(&productos))
}

pub async fn crear(
    State(state): State<AppState>,
    Form(form): Form<ProductoForm>,
) -> Result<Redirect, AppError> {
    let (nombre, precio) = validar(&form)?;
    state
        .db
        .call(move |c| repository::insertar(c, &nombre, precio))
        .await?;
    Ok(Redirect::to("/productos"))
}

pub async fn editar(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Markup, AppError> {
    let producto = state
        .db
        .call(move |c| repository::por_id(c, id))
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(vista_editar(&producto))
}

pub async fn actualizar(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<ProductoForm>,
) -> Result<Redirect, AppError> {
    let (nombre, precio) = validar(&form)?;
    let filas = state
        .db
        .call(move |c| repository::actualizar(c, id, &nombre, precio))
        .await?;
    if filas == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Redirect::to("/productos"))
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
    Ok(Redirect::to("/productos"))
}

fn campos(producto: Option<&Producto>) -> Markup {
    html! {
        div {
            label class="label" for="nombre" { "Nombre" }
            input class="input w-64" id="nombre" name="nombre" required maxlength="100"
                value=[producto.map(|p| p.nombre.as_str())];
        }
        div {
            label class="label" for="precio" { "Precio (S/)" }
            input class="input w-28" id="precio" name="precio" required inputmode="decimal"
                value=[producto.map(|p| format!("{:.2}", p.precio_unitario))];
        }
    }
}

fn vista_lista(productos: &[Producto]) -> Markup {
    layout::page(
        "Productos",
        "productos",
        html! {
            (page_header("Productos", Some("El precio se congela en cada consumo: cambiarlo no altera ventas pasadas.")))
            form method="post" action="/productos" class="card mb-6 flex flex-wrap items-end gap-4" {
                (campos(None))
                button type="submit" class="btn-primary" { "Agregar producto" }
            }
            div class="card overflow-x-auto p-0" {
                table class="table-base" {
                    thead { tr { th { "Producto" } th class="num" { "Precio" } th { "Estado" } th {} } }
                    tbody {
                        @for p in productos {
                            tr class=(if p.esta_activo { "" } else { "text-slate-400" }) {
                                td { (p.nombre) }
                                td class="num" { (soles(p.precio_unitario)) }
                                td { (badge_estado(p.esta_activo)) }
                                td class="space-x-3 text-right whitespace-nowrap" {
                                    a class="btn-link" href=(format!("/productos/{}/editar", p.id)) { "Editar" }
                                    (boton_estado(&format!("/productos/{}/estado", p.id), p.esta_activo))
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

fn vista_editar(producto: &Producto) -> Markup {
    layout::page(
        "Editar producto",
        "productos",
        html! {
            (page_header("Editar producto", Some(&producto.nombre)))
            form method="post" action=(format!("/productos/{}", producto.id)) class="card flex flex-wrap items-end gap-4" {
                (campos(Some(producto)))
                button type="submit" class="btn-primary" { "Guardar" }
                a href="/productos" class="btn-secondary" { "Cancelar" }
            }
        },
    )
}
