use axum::{
    Form,
    extract::{Path, Query, State},
    response::Redirect,
};
use chrono::NaiveDate;
use maud::{Markup, html};
use serde::Deserialize;
use tokio_rusqlite::rusqlite;

use super::{Pago, repository};
use crate::{
    consumos::{self, repository::ConsumoDetalle},
    error::AppError,
    estudiantes::{self, Estudiante},
    fechas, forms,
    state::AppState,
    templates::{
        components::{page_header, soles, soles_o_guion, stat},
        layout,
    },
};

#[derive(Deserialize)]
pub struct SemanaQuery {
    semana: Option<String>,
}

#[derive(Deserialize)]
pub struct PagoForm {
    monto: String,
    fecha: String,
}

#[derive(Deserialize)]
pub struct EliminarForm {
    semana: String,
}

struct Cuenta {
    estudiante: Estudiante,
    deuda_anterior: f64,
    consumos: Vec<ConsumoDetalle>,
    pagos: Vec<Pago>,
}

/// Weekly statement of one student: previous debt, consumption, payments, balance.
pub async fn cuenta(
    State(state): State<AppState>,
    Path(id_estudiante): Path<i64>,
    Query(q): Query<SemanaQuery>,
) -> Result<Markup, AppError> {
    let lunes = fechas::lunes_de(fechas::parse_o_hoy(q.semana.as_deref()));
    let domingo = fechas::sumar_dias(lunes, 6);

    let cuenta = state
        .db
        .call(move |c| {
            let Some(estudiante) = estudiantes::repository::por_id(c, id_estudiante)? else {
                return Ok::<_, rusqlite::Error>(None);
            };
            let deuda_anterior = consumos::repository::total_antes_de(c, id_estudiante, lunes)?
                - repository::total_antes_de(c, id_estudiante, lunes)?;
            Ok(Some(Cuenta {
                estudiante,
                deuda_anterior,
                consumos: consumos::repository::de_estudiante_entre(
                    c,
                    id_estudiante,
                    lunes,
                    domingo,
                )?,
                pagos: repository::de_estudiante_entre(c, id_estudiante, lunes, domingo)?,
            }))
        })
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(vista_cuenta(&cuenta, lunes))
}

pub async fn registrar(
    State(state): State<AppState>,
    Path(id_estudiante): Path<i64>,
    Form(form): Form<PagoForm>,
) -> Result<Redirect, AppError> {
    let monto = forms::monto(&form.monto, "Monto")?;
    let fecha = forms::fecha(&form.fecha, "Fecha")?;

    let registrado = state
        .db
        .call(move |c| {
            if estudiantes::repository::por_id(c, id_estudiante)?.is_none() {
                return Ok::<_, rusqlite::Error>(false);
            }
            repository::registrar(c, id_estudiante, monto, fecha)?;
            Ok(true)
        })
        .await?;
    if !registrado {
        return Err(AppError::NotFound);
    }
    Ok(Redirect::to(&url_cuenta(id_estudiante, fecha)))
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
    Ok(Redirect::to(&url_cuenta(id_estudiante, semana)))
}

pub fn url_cuenta(id_estudiante: i64, semana: NaiveDate) -> String {
    format!(
        "/pagos/{id_estudiante}?semana={}",
        fechas::iso(fechas::lunes_de(semana))
    )
}

struct FilaProducto<'a> {
    id_producto: i64,
    nombre: &'a str,
    cantidades: [i64; 7],
    total: f64,
}

fn vista_cuenta(cuenta: &Cuenta, lunes: NaiveDate) -> Markup {
    let mut filas: Vec<FilaProducto> = Vec::new();
    let mut total_por_dia = [0.0_f64; 7];
    for consumo in &cuenta.consumos {
        let dia = fechas::indice_dia(consumo.fecha);
        total_por_dia[dia] += consumo.total_linea;
        let posicion = match filas
            .iter()
            .position(|f| f.id_producto == consumo.id_producto)
        {
            Some(posicion) => posicion,
            None => {
                filas.push(FilaProducto {
                    id_producto: consumo.id_producto,
                    nombre: &consumo.nombre_producto,
                    cantidades: [0; 7],
                    total: 0.0,
                });
                filas.len() - 1
            }
        };
        filas[posicion].cantidades[dia] += consumo.cantidad;
        filas[posicion].total += consumo.total_linea;
    }

    let consumo_semana: f64 = total_por_dia.iter().sum();
    let pagos_semana: f64 = cuenta.pagos.iter().map(|p| p.monto).sum();
    let saldo = cuenta.deuda_anterior + consumo_semana - pagos_semana;
    // The kiosk works Monday–Saturday; show Sunday only when it has data.
    let dias = if total_por_dia[6] != 0.0 { 7 } else { 6 };
    let id = cuenta.estudiante.id;

    layout::page(
        &cuenta.estudiante.nombre_completo(),
        "inicio",
        html! {
            a class="btn-link mb-2 inline-block" href=(format!("/?semana={}", fechas::iso(lunes))) { "← Volver al inicio" }
            (page_header(&cuenta.estudiante.nombre_completo(), Some(&format!("{} · {}", cuenta.estudiante.grado(), fechas::etiqueta_semana(lunes)))))
            div class="mb-4 flex gap-1" {
                a class="btn-secondary" href=(url_cuenta(id, fechas::sumar_dias(lunes, -7))) { "← Semana anterior" }
                a class="btn-secondary" href=(url_cuenta(id, fechas::sumar_dias(lunes, 7))) { "Semana siguiente →" }
                a class="btn-link ml-2 self-center" href=(url_cuenta(id, fechas::hoy())) { "Esta semana" }
            }
            div class="mb-6 grid gap-3 sm:grid-cols-4" {
                (stat("Deuda anterior", &soles(cuenta.deuda_anterior), false))
                (stat("Consumo de la semana", &soles(consumo_semana), false))
                (stat("Pagos de la semana", &soles(pagos_semana), false))
                (stat(if saldo < -0.005 { "Saldo a favor" } else { "Saldo" }, &soles(saldo.abs()), saldo > 0.005))
            }

            section class="card mb-6 overflow-x-auto p-0" {
                h2 class="px-4 pt-4 pb-2 font-semibold" { "Consumos" }
                @if filas.is_empty() {
                    p class="px-4 pb-4 text-sm text-slate-500" { "Sin consumos esta semana." }
                } @else {
                    table class="table-base" {
                        thead {
                            tr {
                                th { "Producto" }
                                @for d in 0..dias {
                                    th class="num" { (fechas::DIAS_CORTOS[d]) " " (fechas::corta(fechas::sumar_dias(lunes, d as i64))) }
                                }
                                th class="num" { "Total" }
                            }
                        }
                        tbody {
                            @for fila in &filas {
                                tr {
                                    td { (fila.nombre) }
                                    @for d in 0..dias {
                                        td class="num" {
                                            @if fila.cantidades[d] > 0 { (fila.cantidades[d]) } @else { span class="text-slate-300" { "—" } }
                                        }
                                    }
                                    td class="num" { (soles(fila.total)) }
                                }
                            }
                        }
                        tfoot class="font-semibold" {
                            tr {
                                td { "Total" }
                                @for d in 0..dias { td class="num" { (soles_o_guion(total_por_dia[d])) } }
                                td class="num" { (soles(consumo_semana)) }
                            }
                        }
                    }
                }
            }

            div class="grid gap-6 md:grid-cols-2" {
                section class="card" {
                    h2 class="mb-3 font-semibold" { "Registrar pago" }
                    form method="post" action=(format!("/pagos/{id}")) class="flex flex-wrap items-end gap-3" {
                        div {
                            label class="label" for="monto" { "Monto (S/)" }
                            input class="input w-28" id="monto" name="monto" required inputmode="decimal"
                                value=[(saldo > 0.005).then(|| format!("{saldo:.2}"))];
                        }
                        div {
                            label class="label" for="fecha" { "Fecha" }
                            input class="input" type="date" id="fecha" name="fecha" required value=(fechas::iso(lunes));
                        }
                        button type="submit" class="btn-primary" { "Registrar" }
                    }
                }
                section class="card" {
                    h2 class="mb-3 font-semibold" { "Pagos de la semana" }
                    @if cuenta.pagos.is_empty() {
                        p class="text-sm text-slate-500" { "Sin pagos esta semana." }
                    } @else {
                        ul class="divide-y divide-slate-100 text-sm" {
                            @for pago in &cuenta.pagos {
                                li class="flex items-center justify-between py-2" {
                                    span { (fechas::larga(pago.fecha)) }
                                    span class="flex items-center gap-4" {
                                        span class="font-medium tabular-nums" { (soles(pago.monto)) }
                                        form method="post" action=(format!("/pagos/{id}/{}/eliminar", pago.id))
                                            onsubmit="return confirm('¿Eliminar este pago?')" {
                                            input type="hidden" name="semana" value=(fechas::iso(lunes));
                                            button type="submit" class="btn-danger" { "Eliminar" }
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
