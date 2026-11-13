use axum::extract::{Query, State};
use chrono::NaiveDate;
use maud::{Markup, html};
use serde::Deserialize;
use tokio_rusqlite::rusqlite;

use super::repository::{self, PagoDelDia, Totales, VentaProducto};
use crate::{
    error::AppError,
    fechas,
    pagos::handler::url_pagos,
    state::AppState,
    templates::{
        components::{Icono, estado_vacio, ios_nav, selector_dia, soles},
        layout,
    },
};

#[derive(Deserialize)]
pub struct DiaQuery {
    fecha: Option<String>,
}

/// `/cierre`: sold and collected on one day.
pub async fn index(
    State(state): State<AppState>,
    Query(q): Query<DiaQuery>,
) -> Result<Markup, AppError> {
    let fecha = fechas::parse_o_hoy(q.fecha.as_deref());
    let (totales, productos, pagos) = state
        .db
        .call(move |c| {
            Ok::<_, rusqlite::Error>((
                repository::totales(c, fecha)?,
                repository::por_producto(c, fecha)?,
                repository::pagos(c, fecha)?,
            ))
        })
        .await?;
    Ok(vista(fecha, &totales, &productos, &pagos))
}

fn tarjeta(etiqueta: &str, valor: &str, color: &str) -> Markup {
    html! {
        div class="ios-card p-4" {
            p class="text-[12px] font-bold tracking-wide text-ios-gray uppercase" { (etiqueta) }
            p class=(format!("mt-1 text-[22px] font-black tabular-nums {color}")) { (valor) }
        }
    }
}

fn vista(
    fecha: NaiveDate,
    totales: &Totales,
    productos: &[VentaProducto],
    pagos: &[PagoDelDia],
) -> Markup {
    let cobrado: f64 = pagos.iter().map(|p| p.monto).sum();
    layout::page(
        "Cierre del día",
        None,
        html! {
            (ios_nav(("/", "Atrás"), "Cierre del día", html! {
                button type="button" onclick="window.print()" class="ios-link text-[15px] font-medium" { "Imprimir" }
            }))
            (selector_dia(fecha, |d| format!("/cierre?fecha={}", fechas::iso(d))))
            main class="mx-auto max-w-2xl px-4 pb-10 lg:max-w-4xl" {
                header class="mb-6" {
                    h1 class="text-[34px] leading-tight font-bold tracking-tight text-gray-900" { "Cierre del día" }
                    p class="mt-1 text-[17px] font-medium text-ios-gray" { (fechas::larga(fecha)) }
                }
                div class="mb-6 grid grid-cols-2 gap-3 lg:grid-cols-4" {
                    (tarjeta("Vendido", &soles(totales.vendido), "text-gray-900"))
                    (tarjeta("Cobrado", &soles(cobrado), "text-ios-green"))
                    (tarjeta("Alumnos atendidos", &totales.atendidos.to_string(), "text-gray-900"))
                    (tarjeta("Unidades", &totales.unidades.to_string(), "text-gray-900"))
                }
                div class="ios-card mb-6 divide-y divide-gray-100 text-[15px]" {
                    div class="flex justify-between px-5 py-3" { span class="text-gray-500" { "Sector Menor" } span class="font-semibold tabular-nums" { (soles(totales.sector_menor)) } }
                    div class="flex justify-between px-5 py-3" { span class="text-gray-500" { "Sector Mayor" } span class="font-semibold tabular-nums" { (soles(totales.sector_mayor)) } }
                }
                div class="grid gap-6 lg:grid-cols-2" {
                    section {
                        h3 class="section-title" { "Ventas por producto" }
                        div class="ios-card" {
                            @if productos.is_empty() {
                                (estado_vacio(Icono::Productos, "Sin ventas", "No hay consumos registrados en este día."))
                            } @else {
                                div class="divide-y divide-gray-100" {
                                    @for p in productos {
                                        div class="flex items-center justify-between px-5 py-3 text-[15px]" {
                                            span class="font-medium text-gray-900" { (p.nombre) span class="ml-2 font-bold text-ios-blue" { "×" (p.unidades) } }
                                            span class="font-semibold tabular-nums" { (soles(p.total)) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    section {
                        h3 class="section-title" { "Pagos registrados con esta fecha" }
                        div class="ios-card" {
                            @if pagos.is_empty() {
                                (estado_vacio(Icono::Dinero, "Sin pagos", "No hay pagos con esta fecha."))
                            } @else {
                                div class="divide-y divide-gray-100" {
                                    @for p in pagos {
                                        a href=(url_pagos(p.id_estudiante, fecha, None))
                                            class="flex items-center justify-between px-5 py-3 text-[15px] hover:bg-gray-50" {
                                            span class="truncate font-medium text-gray-900" { (p.estudiante) }
                                            span class="shrink-0 font-semibold text-ios-green tabular-nums" { (soles(p.monto)) }
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
