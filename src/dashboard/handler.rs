use axum::extract::{Query, State};
use chrono::NaiveDate;
use maud::{Markup, html};
use serde::Deserialize;
use tokio_rusqlite::rusqlite;

use super::repository;
use crate::{
    error::AppError,
    estudiantes::{self, Estudiante, grado_valido, handler::select_grado},
    fechas,
    pagos::handler::url_cuenta,
    state::AppState,
    templates::{
        components::{page_header, soles, soles_o_guion},
        layout,
    },
};

#[derive(Deserialize)]
pub struct Filtro {
    semana: Option<String>,
    grado: Option<String>,
}

struct Fila {
    estudiante: Estudiante,
    deuda_anterior: f64,
    por_dia: [f64; 7],
    consumo: f64,
    pagos: f64,
}

impl Fila {
    fn saldo(&self) -> f64 {
        self.deuda_anterior + self.consumo - self.pagos
    }
}

/// Main view: weekly balance of every active student.
pub async fn index(
    State(state): State<AppState>,
    Query(filtro): Query<Filtro>,
) -> Result<Markup, AppError> {
    let lunes = fechas::lunes_de(fechas::parse_o_hoy(filtro.semana.as_deref()));
    let domingo = fechas::sumar_dias(lunes, 6);
    let id_grado = filtro
        .grado
        .and_then(|g| g.parse().ok())
        .filter(|g| grado_valido(*g))
        .unwrap_or(0);

    let (estudiantes, deudas, pagos, consumos) = state
        .db
        .call(move |c| {
            Ok::<_, rusqlite::Error>((
                estudiantes::repository::activos_por_grado(c, id_grado)?,
                repository::deudas_anteriores(c, id_grado, lunes)?,
                repository::pagos_entre(c, id_grado, lunes, domingo)?,
                repository::consumo_por_dia(c, lunes, domingo)?,
            ))
        })
        .await?;

    let filas: Vec<Fila> = estudiantes
        .into_iter()
        .map(|estudiante| {
            let por_dia: [f64; 7] = std::array::from_fn(|d| {
                let dia = fechas::sumar_dias(lunes, d as i64);
                consumos.get(&(estudiante.id, dia)).copied().unwrap_or(0.0)
            });
            Fila {
                deuda_anterior: deudas.get(&estudiante.id).copied().unwrap_or(0.0),
                pagos: pagos.get(&estudiante.id).copied().unwrap_or(0.0),
                consumo: por_dia.iter().sum(),
                por_dia,
                estudiante,
            }
        })
        .collect();

    Ok(vista(lunes, id_grado, &filas))
}

fn vista(lunes: NaiveDate, id_grado: i64, filas: &[Fila]) -> Markup {
    let enlace = |semana: NaiveDate| format!("/?semana={}&grado={id_grado}", fechas::iso(semana));
    // The kiosk works Monday–Saturday; show Sunday only when it has data.
    let dias = if filas.iter().any(|f| f.por_dia[6] != 0.0) {
        7
    } else {
        6
    };
    let total = |valor: fn(&Fila) -> f64| filas.iter().map(valor).sum::<f64>();
    let total_dia = |d: usize| filas.iter().map(|f| f.por_dia[d]).sum::<f64>();

    layout::page(
        "Inicio",
        "inicio",
        html! {
            (page_header("Cuentas de la semana", Some(&fechas::etiqueta_semana(lunes))))
            div class="mb-4 flex flex-wrap items-end gap-3" {
                form method="get" action="/" class="flex flex-wrap items-end gap-2" {
                    div {
                        label class="label" for="grado" { "Grado" }
                        (select_grado("grado", Some(id_grado), true))
                    }
                    div {
                        label class="label" for="semana" { "Semana de" }
                        input class="input" type="date" id="semana" name="semana" value=(fechas::iso(lunes));
                    }
                    button type="submit" class="btn-primary" { "Ver" }
                }
                div class="flex gap-1" {
                    a class="btn-secondary" href=(enlace(fechas::sumar_dias(lunes, -7))) { "← Anterior" }
                    a class="btn-secondary" href=(enlace(fechas::sumar_dias(lunes, 7))) { "Siguiente →" }
                    a class="btn-link ml-2 self-center" href=(enlace(fechas::hoy())) { "Esta semana" }
                }
            }
            @if filas.is_empty() {
                p class="card text-slate-500" { "No hay estudiantes activos para este filtro." }
            } @else {
                div class="card overflow-x-auto p-0" {
                    table class="table-base" {
                        thead {
                            tr {
                                th { "Estudiante" }
                                @if id_grado == 0 { th { "Grado" } }
                                th class="num" { "Deuda ant." }
                                @for d in 0..dias {
                                    th class="num" { (fechas::DIAS_CORTOS[d]) " " (fechas::corta(fechas::sumar_dias(lunes, d as i64))) }
                                }
                                th class="num" { "Consumo" }
                                th class="num" { "Pagos" }
                                th class="num" { "Saldo" }
                            }
                        }
                        tbody {
                            @for fila in filas {
                                @let saldo = fila.saldo();
                                tr class="hover:bg-slate-50" {
                                    td class="whitespace-nowrap" {
                                        a class="hover:text-emerald-700 hover:underline" href=(url_cuenta(fila.estudiante.id, lunes)) {
                                            (fila.estudiante.nombre_completo())
                                        }
                                    }
                                    @if id_grado == 0 { td class="whitespace-nowrap text-slate-500" { (fila.estudiante.grado()) } }
                                    td class="num" { (soles_o_guion(fila.deuda_anterior)) }
                                    @for d in 0..dias { td class="num" { (soles_o_guion(fila.por_dia[d])) } }
                                    td class="num" { (soles_o_guion(fila.consumo)) }
                                    td class="num" { (soles_o_guion(fila.pagos)) }
                                    td class=(if saldo > 0.005 { "num font-semibold text-red-700" } else { "num font-semibold text-emerald-700" }) {
                                        (soles(saldo))
                                    }
                                }
                            }
                        }
                        tfoot class="font-semibold" {
                            tr {
                                td { (filas.len()) " estudiantes" }
                                @if id_grado == 0 { td {} }
                                td class="num" { (soles(total(|f| f.deuda_anterior))) }
                                @for d in 0..dias { td class="num" { (soles(total_dia(d))) } }
                                td class="num" { (soles(total(|f| f.consumo))) }
                                td class="num" { (soles(total(|f| f.pagos))) }
                                td class="num" { (soles(total(Fila::saldo))) }
                            }
                        }
                    }
                }
            }
        },
    )
}
