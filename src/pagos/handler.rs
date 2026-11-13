use axum::{
    Form,
    extract::{Path, Query, State},
    response::Redirect,
};
use chrono::NaiveDate;
use maud::{Markup, html};
use serde::Deserialize;
use tokio_rusqlite::rusqlite::{self, Connection};

use super::{Pago, repository};
use crate::{
    assets,
    consumos::{self, repository::ConsumoDetalle},
    error::AppError,
    estudiantes::{self, Estudiante, grado_desde},
    fechas, forms,
    state::AppState,
    templates::{
        components::{Icono, estado_vacio, icono, mensaje_flash, monto, soles},
        layout,
    },
};

#[derive(Deserialize)]
pub struct SemanaQuery {
    fecha: Option<String>,
    grado: Option<String>,
    ok: Option<String>,
}

#[derive(Deserialize)]
pub struct PagoForm {
    monto: String,
    fecha_pago: String,
    grado: Option<String>,
    /// One-time token issued with the form (see `envios`).
    token: String,
}

#[derive(Deserialize)]
pub struct EliminarForm {
    semana: String,
    grado: Option<String>,
}

/// Everything about one student for one week.
struct Semana {
    estudiante: Estudiante,
    lunes: NaiveDate,
    deuda_anterior: f64,
    consumos: Vec<ConsumoDetalle>,
    pagos: Vec<Pago>,
}

impl Semana {
    fn subtotal(&self) -> f64 {
        self.consumos.iter().map(|c| c.total_linea).sum()
    }

    fn pagado(&self) -> f64 {
        self.pagos.iter().map(|p| p.monto).sum()
    }

    fn total(&self) -> f64 {
        self.deuda_anterior + self.subtotal() - self.pagado()
    }
}

/// Loads the week (Monday–Sunday) that contains `fecha`.
fn cargar_semana(
    c: &Connection,
    id_estudiante: i64,
    fecha: NaiveDate,
) -> rusqlite::Result<Option<Semana>> {
    let Some(estudiante) = estudiantes::repository::por_id(c, id_estudiante)? else {
        return Ok(None);
    };
    let lunes = fechas::lunes_de(fecha);
    let domingo = fechas::sumar_dias(lunes, 6);
    let deuda_anterior = consumos::repository::total_antes_de(c, id_estudiante, lunes)?
        - repository::total_antes_de(c, id_estudiante, lunes)?;
    Ok(Some(Semana {
        estudiante,
        lunes,
        deuda_anterior,
        consumos: consumos::repository::de_estudiante_entre(c, id_estudiante, lunes, domingo)?,
        pagos: repository::de_estudiante_entre(c, id_estudiante, lunes, domingo)?,
    }))
}

async fn semana(
    state: &AppState,
    id_estudiante: i64,
    fecha: Option<&str>,
) -> Result<Semana, AppError> {
    let fecha = fechas::parse_o_hoy(fecha);
    state
        .db
        .call(move |c| cargar_semana(c, id_estudiante, fecha))
        .await?
        .ok_or(AppError::NotFound)
}

fn url_semana(lunes: NaiveDate, grado: Option<i64>) -> String {
    match grado {
        Some(g) => format!("/?fecha={}&grado={g}", fechas::iso(lunes)),
        None => format!("/?fecha={}", fechas::iso(lunes)),
    }
}

pub fn url_pagos(id_estudiante: i64, fecha: NaiveDate, grado: Option<i64>) -> String {
    let base = format!(
        "/pagos/{id_estudiante}?fecha={}",
        fechas::iso(fechas::lunes_de(fecha))
    );
    match grado {
        Some(g) => format!("{base}&grado={g}"),
        None => base,
    }
}

pub fn url_comprobante(id_estudiante: i64, lunes: NaiveDate, grado: i64) -> String {
    format!(
        "/comprobante/{id_estudiante}?fecha={}&grado={grado}",
        fechas::iso(lunes)
    )
}

/// `/pagos/{id}`: register and review payments of one week.
pub async fn gestion(
    State(state): State<AppState>,
    Path(id_estudiante): Path<i64>,
    Query(q): Query<SemanaQuery>,
) -> Result<Markup, AppError> {
    let semana = semana(&state, id_estudiante, q.fecha.as_deref()).await?;
    let grado = grado_desde(q.grado.as_deref()).or(semana.estudiante.id_grado);
    let token = state.envios_pago.nuevo_token();
    Ok(vista_gestion(
        &semana,
        grado,
        &token,
        mensaje_flash(q.ok.as_deref()),
    ))
}

pub async fn registrar(
    State(state): State<AppState>,
    Path(id_estudiante): Path<i64>,
    Form(form): Form<PagoForm>,
) -> Result<Redirect, AppError> {
    let monto = forms::monto(&form.monto, "Monto")?;
    let fecha = forms::fecha(&form.fecha_pago, "Fecha")?;
    let token = forms::texto(&form.token, "Token", 64)?;
    let grado = grado_desde(form.grado.as_deref());

    // Same form submitted again (double tap, back + resubmit): do not pay twice.
    if !state.envios_pago.consumir(&token) {
        tracing::warn!(id_estudiante, "duplicate payment submission ignored");
        return Ok(Redirect::to(&format!(
            "{}&ok=pago-duplicado",
            url_pagos(id_estudiante, fecha, grado)
        )));
    }

    let resultado = state
        .db
        .call(move |c| {
            if estudiantes::repository::por_id(c, id_estudiante)?.is_none() {
                return Ok::<_, rusqlite::Error>(false);
            }
            repository::registrar(c, id_estudiante, monto, fecha)?;
            Ok(true)
        })
        .await;
    match resultado {
        Ok(true) => Ok(Redirect::to(&format!(
            "{}&ok=pago",
            url_pagos(id_estudiante, fecha, grado)
        ))),
        Ok(false) => {
            state.envios_pago.liberar(&token);
            Err(AppError::NotFound)
        }
        Err(err) => {
            state.envios_pago.liberar(&token);
            Err(err.into())
        }
    }
}

pub async fn eliminar(
    State(state): State<AppState>,
    Path((id_estudiante, id_pago)): Path<(i64, i64)>,
    Form(form): Form<EliminarForm>,
) -> Result<Redirect, AppError> {
    let filas = state
        .db
        .call(move |c| repository::eliminar(c, id_pago, id_estudiante))
        .await?;
    if filas == 0 {
        return Err(AppError::NotFound);
    }
    let semana = fechas::parse_o_hoy(Some(&form.semana));
    let grado = grado_desde(form.grado.as_deref());
    Ok(Redirect::to(&format!(
        "{}&ok=pago-eliminado",
        url_pagos(id_estudiante, semana, grado)
    )))
}

fn color_saldo(total: f64) -> &'static str {
    if total > 0.005 {
        "text-ios-red"
    } else {
        "text-ios-green"
    }
}

fn vista_gestion(semana: &Semana, grado: Option<i64>, token: &str, flash: Option<&str>) -> Markup {
    let id = semana.estudiante.id;
    let total = semana.total();
    let pagado = semana.pagado();
    let nombre = semana.estudiante.nombre_completo();
    let navegar = |dias: i64| url_pagos(id, fechas::sumar_dias(semana.lunes, dias), grado);

    layout::page(
        &format!("Pagos - {nombre}"),
        flash,
        html! {
            nav class="ios-nav" {
                div class="mx-auto flex max-w-2xl items-center justify-between lg:max-w-6xl" {
                    a href=(url_semana(semana.lunes, grado)) class="ios-link" {
                        (icono(Icono::ChevronIzquierda, "-ml-2 h-6 w-6"))
                        span class="text-[17px] font-medium" { "Atrás" }
                    }
                    h2 class="hidden truncate px-4 text-[17px] font-semibold sm:block" { "Gestión de Pagos" }
                    div class="flex items-center gap-2" {
                        span class="text-[13px] font-semibold tracking-wider text-ios-gray uppercase" { "Saldo:" }
                        span class=(format!("text-[17px] font-bold tabular-nums {}", color_saldo(total))) { (soles(total)) }
                    }
                }
            }
            main class="mx-auto max-w-2xl px-4 pt-6 pb-20 lg:max-w-6xl lg:pt-10" {
                header class="mb-8 lg:mb-12" {
                    h1 class="text-[34px] leading-tight font-bold tracking-tight text-gray-900" { (nombre) }
                    p class="mt-1 text-[17px] font-medium text-ios-gray" { (fechas::etiqueta_semana(semana.lunes)) }
                    // Desktop-only extras; mobile keeps the original Go layout.
                    div class="mt-2 hidden items-center gap-3 lg:flex" {
                        a href=(navegar(-7)) class="inline-flex items-center text-[15px] font-semibold text-ios-blue" { (icono(Icono::ChevronIzquierda, "h-4 w-4")) "Semana anterior" }
                        a href=(navegar(7)) class="inline-flex items-center text-[15px] font-semibold text-ios-blue" { "Semana siguiente" (icono(Icono::ChevronDerecha, "h-4 w-4")) }
                        a href=(url_comprobante(id, semana.lunes, grado.unwrap_or(0))) class="ml-auto inline-flex items-center gap-1 text-[15px] font-semibold text-ios-blue" {
                            (icono(Icono::Recibo, "h-5 w-5")) "Comprobante"
                        }
                    }
                }
                div class="lg:grid lg:grid-cols-12 lg:items-start lg:gap-10" {
                    aside class="mb-10 lg:sticky lg:top-24 lg:col-span-5 lg:mb-0" {
                        h3 class="section-title font-medium" { "Registrar nuevo pago" }
                        div class="ios-card" {
                            form method="post" action=(format!("/pagos/{id}")) autocomplete="off" class="divide-y divide-gray-100" {
                                input type="hidden" name="token" value=(token);
                                @if let Some(g) = grado { input type="hidden" name="grado" value=(g); }
                                div class="ios-row" {
                                    label for="monto" class="ios-label text-gray-600" { "Monto" }
                                    div class="flex flex-1 items-center" {
                                        span class="mr-1 text-[19px] font-semibold text-gray-400" { "S/" }
                                        input type="number" name="monto" id="monto" step="0.01" min="0.01" placeholder="0.00" required inputmode="decimal" autofocus
                                            class="ios-input text-[20px] font-bold tabular-nums text-ios-blue";
                                    }
                                }
                                div class="ios-row" {
                                    label for="fecha_pago" class="ios-label text-gray-600" { "Fecha" }
                                    input type="date" name="fecha_pago" id="fecha_pago" required value=(fechas::iso(semana.lunes))
                                        class="ios-input font-semibold text-ios-blue";
                                }
                                div class="bg-gray-50/50 p-4" {
                                    button type="submit" class="btn-ios" { "Registrar Abono" }
                                }
                            }
                        }
                        div class="mt-6 hidden space-y-2 rounded-3xl border border-gray-200 bg-white p-5 text-[15px] lg:block" {
                            div class="flex justify-between" { span class="text-gray-500" { "Consumo de la semana" } span class="font-semibold tabular-nums" { (soles(semana.subtotal())) } }
                            div class="flex justify-between" { span class="text-gray-500" { "Deuda anterior" } span class="font-semibold tabular-nums" { (soles(semana.deuda_anterior)) } }
                            div class="flex justify-between" { span class="text-gray-500" { "Pagos de la semana" } span class="font-semibold tabular-nums" { "- " (soles(pagado)) } }
                            div class="flex items-center justify-between border-t border-gray-100 pt-2" {
                                span class="font-medium text-gray-700" { "Saldo actual" }
                                span class=(format!("text-[20px] font-black tabular-nums {}", color_saldo(total))) { (soles(total)) }
                            }
                        }
                    }
                    section class="lg:col-span-7" {
                        div class="mb-3 flex items-end justify-between px-4" {
                            h3 class="text-[13px] font-medium tracking-wide text-ios-gray uppercase" { "Histórico de pagos" }
                            @if !semana.pagos.is_empty() {
                                div class="text-right" {
                                    span class="block text-[11px] font-bold text-gray-400 uppercase" { "Total abonado" }
                                    span class="text-[17px] font-bold text-ios-green" { (soles(pagado)) }
                                }
                            }
                        }
                        div class="ios-card" {
                            @if semana.pagos.is_empty() {
                                (estado_vacio(Icono::Dinero, "Historial vacío", "Aún no se han registrado pagos para esta semana."))
                            } @else {
                                div class="divide-y divide-gray-100" {
                                    @for pago in &semana.pagos {
                                        div class="flex items-center justify-between p-5 transition-colors hover:bg-gray-50" {
                                            div class="flex items-center gap-4" {
                                                div class="flex h-12 w-12 shrink-0 items-center justify-center rounded-full bg-ios-green-soft" {
                                                    (icono(Icono::Check, "h-6 w-6 text-ios-green"))
                                                }
                                                div {
                                                    p class="text-[19px] leading-tight font-bold text-gray-900 tabular-nums" { (soles(pago.monto)) }
                                                    p class="mt-0.5 text-[14px] text-ios-gray" { (fechas::larga(pago.fecha)) }
                                                }
                                            }
                                            form method="post" action=(format!("/pagos/{id}/{}/eliminar", pago.id)) data-confirm="¿Estás seguro de eliminar este pago?" {
                                                input type="hidden" name="semana" value=(fechas::iso(semana.lunes));
                                                @if let Some(g) = grado { input type="hidden" name="grado" value=(g); }
                                                button type="submit" class="-mr-2 px-3 py-2 text-[15px] font-semibold text-ios-red transition-colors hover:text-red-700" { "Eliminar" }
                                            }
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

/// `/comprobante/{id}`: weekly sales note that can be copied as an image.
pub async fn comprobante(
    State(state): State<AppState>,
    Path(id_estudiante): Path<i64>,
    Query(q): Query<SemanaQuery>,
) -> Result<Markup, AppError> {
    let semana = semana(&state, id_estudiante, q.fecha.as_deref()).await?;
    let grado = grado_desde(q.grado.as_deref()).or(semana.estudiante.id_grado);
    Ok(vista_comprobante(&semana, grado))
}

fn vista_comprobante(semana: &Semana, grado: Option<i64>) -> Markup {
    // Consumption arrives ordered by date: group consecutive lines per day.
    let mut por_dia: Vec<Vec<&ConsumoDetalle>> = Vec::new();
    for consumo in &semana.consumos {
        match por_dia.last_mut() {
            Some(dia) if dia[0].fecha == consumo.fecha => dia.push(consumo),
            _ => por_dia.push(vec![consumo]),
        }
    }
    let nombre = semana.estudiante.nombre_completo();

    layout::page(
        &format!("Comprobante - {nombre}"),
        None,
        html! {
            script src=(assets::url("js/vendor/html2canvas.min.js")) defer {}
            main class="mx-auto max-w-2xl p-2 sm:p-4" {
                div class="mb-4 flex items-center justify-between gap-3" {
                    a href=(url_semana(semana.lunes, grado)) class="rounded-lg border border-gray-200/50 bg-white/70 px-4 py-2.5 text-sm font-medium text-gray-700 shadow-sm backdrop-blur-md transition-all duration-150 hover:bg-white/90 active:scale-95" { "← Volver" }
                    button type="button" data-copy-receipt data-file-name=(format!("nota-venta-{nombre}.png"))
                        class="flex items-center gap-2 rounded-lg border border-blue-700/20 bg-gradient-to-b from-blue-500 to-blue-600 px-4 py-2.5 text-sm font-medium text-white shadow-md transition-all duration-150 hover:from-blue-600 hover:to-blue-700 active:scale-95 disabled:cursor-not-allowed disabled:opacity-60" {
                        span { "📋 Copiar" }
                    }
                }
                div id="comprobante" class="overflow-hidden rounded-lg border-2 border-gray-800 bg-white shadow-lg" {
                    div class="border-b-2 border-gray-800 px-4 py-3 text-center" {
                        h1 class="text-xl font-bold tracking-wide text-gray-900 uppercase" { "Kiosco Escolar" }
                        p class="mt-0.5 text-xs font-semibold text-gray-600" { "Nota de Venta Semanal" }
                    }
                    div class="space-y-1 border-b-2 border-dashed border-gray-400 bg-gray-50 px-4 py-3 text-xs" {
                        div class="flex justify-between gap-4" {
                            span class="text-[10px] font-medium text-gray-600 uppercase" { "Cliente:" }
                            span class="text-right font-bold text-gray-900" { (nombre) }
                        }
                        div class="flex justify-between gap-4" {
                            span class="text-[10px] font-medium text-gray-600 uppercase" { "Grado:" }
                            span class="font-medium text-gray-800" { (semana.estudiante.grado()) }
                        }
                        div class="flex justify-between gap-4" {
                            span class="text-[10px] font-medium text-gray-600 uppercase" { "Periodo:" }
                            span class="font-medium text-gray-800" { (fechas::periodo(semana.lunes)) }
                        }
                    }
                    div class="px-4 py-3" {
                        @if por_dia.is_empty() {
                            p class="py-4 text-center text-xs text-gray-500" { "No hay consumos en esta semana" }
                        } @else {
                            div class="grid grid-cols-2 gap-2" {
                                @for dia in &por_dia {
                                    div class="rounded border-2 border-gray-300" {
                                        div class="flex items-center justify-between border-b-2 border-gray-300 bg-gray-100 px-2 py-1" {
                                            span class="text-xs font-bold text-gray-900 uppercase" { (fechas::corta(dia[0].fecha)) }
                                            span class="text-xs font-bold text-gray-900" { (soles(dia.iter().map(|c| c.total_linea).sum())) }
                                        }
                                        div class="divide-y divide-gray-100" {
                                            @for linea in dia {
                                                div class="flex items-center justify-between px-2 py-1 text-xs" {
                                                    div class="flex min-w-0 items-center gap-1" {
                                                        span class="truncate text-gray-700" { (linea.nombre_producto) }
                                                        span class="shrink-0 text-[10px] text-gray-400" { "× " (linea.cantidad) }
                                                    }
                                                    span class="ml-1 shrink-0 font-semibold text-gray-900" { (monto(linea.total_linea)) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div class="px-4 pb-3" {
                        div class="space-y-1.5 rounded border-2 border-gray-800 bg-white p-3 text-xs" {
                            div class="flex justify-between py-0.5" { span class="font-medium text-gray-700 uppercase" { "Subtotal semana:" } span class="font-bold text-gray-900 tabular-nums" { (soles(semana.subtotal())) } }
                            div class="flex justify-between py-0.5" { span class="font-medium text-gray-700 uppercase" { "Deuda anterior:" } span class="font-bold text-gray-900 tabular-nums" { (soles(semana.deuda_anterior)) } }
                            div class="flex justify-between py-0.5" { span class="font-medium text-gray-700 uppercase" { "Pagos realizados:" } span class="font-bold text-gray-900 tabular-nums" { "- " (soles(semana.pagado())) } }
                            div class="mt-2 flex items-center justify-between border-t-2 border-gray-800 pt-2" {
                                span class="text-sm font-bold text-gray-900 uppercase" { "Total a pagar:" }
                                span class="text-lg font-black text-gray-900 tabular-nums" { (soles(semana.total())) }
                            }
                        }
                    }
                    div class="border-t-2 border-gray-800 bg-gray-100 px-3 py-2" {
                        p class="text-center text-[10px] font-medium tracking-wide text-gray-600 uppercase" { "Nota de venta - Semana del " (fechas::larga(semana.lunes)) }
                        p class="mt-0.5 text-center text-[9px] text-gray-500" { "Gracias por su preferencia" }
                    }
                }
            }
        },
    )
}
