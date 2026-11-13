use axum::{
    Form,
    extract::{Path, Query, State},
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
        components::{
            Icono, bottom_nav, encabezado, estado_vacio, icono, ios_nav, mensaje_flash, monto,
            soles,
        },
        layout,
    },
};

#[derive(Deserialize)]
pub struct ListaQuery {
    editar: Option<i64>,
    ok: Option<String>,
}

#[derive(Deserialize)]
pub struct ProductoForm {
    nombre: String,
    precio_unitario: String,
}

#[derive(Deserialize)]
pub struct EstadoForm {
    activo: u8,
}

fn validar(form: &ProductoForm) -> Result<(String, f64), AppError> {
    Ok((
        forms::texto(&form.nombre, "Nombre", 100)?,
        forms::monto(&form.precio_unitario, "Precio")?,
    ))
}

pub async fn listar(
    State(state): State<AppState>,
    Query(q): Query<ListaQuery>,
) -> Result<Markup, AppError> {
    let productos = state.db.call(|c| repository::todos(c)).await?;
    Ok(vista(&productos, q.editar, mensaje_flash(q.ok.as_deref())))
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
    Ok(Redirect::to("/productos?ok=producto-creado"))
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
    Ok(Redirect::to("/productos?ok=producto-actualizado"))
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
    Ok(Redirect::to("/productos?ok=producto-estado"))
}

fn fila(p: &Producto) -> Markup {
    html! {
        div class="flex items-center justify-between gap-2 p-4 transition-colors hover:bg-gray-50" {
            div class="flex min-w-0 items-center gap-4" {
                div class=(if p.esta_activo { "flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-indigo-50 text-indigo-600" } else { "flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-gray-100 text-gray-400 opacity-50" }) {
                    (icono(Icono::Productos, "h-5 w-5"))
                }
                div class="min-w-0" {
                    p class=(if p.esta_activo { "truncate text-[17px] leading-tight font-semibold text-gray-900" } else { "truncate text-[17px] leading-tight font-semibold text-gray-400 line-through" }) {
                        (p.nombre)
                    }
                    p class=(if p.esta_activo { "text-[15px] font-medium tabular-nums text-ios-blue" } else { "text-[15px] font-medium tabular-nums text-gray-400" }) {
                        (soles(p.precio_unitario))
                    }
                }
            }
            div class="flex shrink-0 items-center gap-1" {
                a href=(format!("/productos?editar={}#p{}", p.id, p.id)) class="btn-text text-ios-blue hover:bg-blue-50" { "Editar" }
                form method="post" action=(format!("/productos/{}/estado", p.id))
                    data-confirm=[p.esta_activo.then(|| format!("¿Deshabilitar {}?", p.nombre))] {
                    input type="hidden" name="activo" value=(if p.esta_activo { "0" } else { "1" });
                    @if p.esta_activo {
                        button type="submit" class="btn-text text-ios-red hover:bg-red-50" { "Desactivar" }
                    } @else {
                        button type="submit" class="btn-text text-ios-green hover:bg-green-50" { "Activar" }
                    }
                }
            }
        }
    }
}

fn fila_edicion(p: &Producto) -> Markup {
    html! {
        div class="border-l-4 border-ios-blue bg-[#f9f9f9] p-5" {
            form method="post" action=(format!("/productos/{}", p.id)) class="space-y-4" {
                div class="grid grid-cols-1 gap-4 sm:grid-cols-2" {
                    label class="block rounded-xl border border-gray-200 bg-white p-3" {
                        span class="mb-1 block text-[12px] font-bold text-gray-400 uppercase" { "Nombre del producto" }
                        input type="text" name="nombre" value=(p.nombre) required maxlength="100" autofocus
                            class="w-full border-none bg-transparent p-0 text-[17px] font-medium text-gray-900";
                    }
                    label class="block rounded-xl border border-gray-200 bg-white p-3" {
                        span class="mb-1 block text-[12px] font-bold text-gray-400 uppercase" { "Precio (S/)" }
                        input type="number" name="precio_unitario" value=(monto(p.precio_unitario)) step="0.01" min="0.01" required inputmode="decimal"
                            class="w-full border-none bg-transparent p-0 text-[17px] font-bold text-ios-blue";
                    }
                }
                p class="text-[13px] text-ios-gray" { "Cambiar el precio no altera las ventas ya registradas." }
                div class="flex gap-3" {
                    button type="submit" class="flex-1 rounded-xl bg-ios-blue py-3 font-bold text-white shadow-sm transition-all active:scale-[0.98]" { "Guardar Cambios" }
                    a href="/productos" class="rounded-xl border border-gray-200 bg-white px-6 py-3 font-semibold text-gray-600 transition-all active:scale-[0.98]" { "Cancelar" }
                }
            }
        }
    }
}

fn vista(productos: &[Producto], editar: Option<i64>, flash: Option<&str>) -> Markup {
    let activos = productos.iter().filter(|p| p.esta_activo).count();
    layout::page(
        "Productos",
        flash,
        html! {
            (bottom_nav())
            (ios_nav(("/", "Atrás"), "Configuración de Productos", html! {}))
            main class="mx-auto max-w-2xl px-4 pt-6 pb-32 sm:pb-10 lg:max-w-6xl" {
                (encabezado("Productos", "Catálogo general del kiosco"))
                div class="lg:grid lg:grid-cols-12 lg:items-start lg:gap-10" {
                    aside class="mb-10 lg:sticky lg:top-24 lg:col-span-5 lg:mb-0" {
                        h3 class="section-title font-medium" { "Nuevo producto" }
                        div class="ios-card" {
                            form method="post" action="/productos" autocomplete="off" class="divide-y divide-gray-100" {
                                div class="ios-row" {
                                    label for="new-name" class="ios-label" { "Nombre" }
                                    input id="new-name" type="text" name="nombre" placeholder="Ej. Gelatina" required maxlength="100" class="ios-input";
                                }
                                div class="ios-row" {
                                    label for="new-price" class="ios-label" { "Precio" }
                                    div class="flex flex-1 items-center" {
                                        span class="mr-1 text-[17px] text-gray-400" { "S/" }
                                        input id="new-price" type="number" name="precio_unitario" step="0.01" min="0.01" placeholder="0.00" required inputmode="decimal"
                                            class="ios-input font-bold text-ios-blue";
                                    }
                                }
                                div class="bg-gray-50/50 p-4" {
                                    button type="submit" class="btn-ios" { "Agregar al Catálogo" }
                                }
                            }
                        }
                    }
                    section class="lg:col-span-7" {
                        div class="mb-3 flex items-center justify-between px-4" {
                            h3 class="text-[13px] font-medium tracking-wide text-ios-gray uppercase" { "Productos registrados" }
                            span class="badge bg-indigo-50 text-indigo-600" { (activos) " activos de " (productos.len()) }
                        }
                        div class="ios-card" {
                            @if productos.is_empty() {
                                (estado_vacio(Icono::Productos, "Sin productos", "Comienza agregando productos en el panel lateral."))
                            } @else {
                                div class="divide-y divide-gray-200/70" {
                                    @for p in productos {
                                        div id=(format!("p{}", p.id)) class="scroll-mt-24" {
                                            @if editar == Some(p.id) { (fila_edicion(p)) } @else { (fila(p)) }
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
