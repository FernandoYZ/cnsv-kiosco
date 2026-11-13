use std::collections::HashMap;

use axum::extract::{Query, State};
use chrono::NaiveDate;
use maud::{Markup, html};
use serde::Deserialize;
use tokio_rusqlite::rusqlite;

use super::repository::{self, LineaSemana};
use crate::{
    error::AppError,
    estudiantes::{self, Estudiante, GRADOS, grado_desde},
    fechas, rutas,
    state::AppState,
    templates::{
        components::{Icono, bottom_nav, es_cero, icono, mensaje_flash, monto, soles},
        layout,
    },
};

#[derive(Deserialize)]
pub struct Filtro {
    fecha: Option<String>,
    grado: Option<String>,
    ok: Option<String>,
}

#[derive(Default)]
struct Dia<'a> {
    /// (product, quantity), merged when legacy data has duplicate rows.
    productos: Vec<(&'a str, i64)>,
    total: f64,
}

struct Fila<'a> {
    estudiante: Estudiante,
    dias: [Dia<'a>; 7],
    deuda: f64,
    pagos: f64,
}

impl Fila<'_> {
    fn subtotal(&self) -> f64 {
        self.dias.iter().map(|d| d.total).sum()
    }

    fn total(&self) -> f64 {
        self.subtotal() + self.deuda - self.pagos
    }
}

/// Main view: weekly consumption of one grade, as in the original design.
pub async fn index(
    State(state): State<AppState>,
    Query(filtro): Query<Filtro>,
) -> Result<Markup, AppError> {
    let lunes = fechas::lunes_de(fechas::parse_o_hoy(filtro.fecha.as_deref()));
    let domingo = fechas::sumar_dias(lunes, 6);
    let id_grado = grado_desde(filtro.grado.as_deref()).unwrap_or(GRADOS[0].id);

    let (estudiantes, deudas, pagos, lineas) = state
        .db
        .call(move |c| {
            Ok::<_, rusqlite::Error>((
                estudiantes::repository::activos_por_grado(c, id_grado)?,
                repository::deudas_anteriores(c, id_grado, lunes)?,
                repository::pagos_entre(c, id_grado, lunes, domingo)?,
                repository::lineas_semana(c, id_grado, lunes, domingo)?,
            ))
        })
        .await?;

    let mut por_estudiante: HashMap<i64, [Dia; 7]> = HashMap::new();
    for linea in &lineas {
        agregar(
            por_estudiante.entry(linea.id_estudiante).or_default(),
            linea,
        );
    }
    let filas: Vec<Fila> = estudiantes
        .into_iter()
        .map(|estudiante| Fila {
            deuda: deudas.get(&estudiante.id).copied().unwrap_or(0.0),
            pagos: pagos.get(&estudiante.id).copied().unwrap_or(0.0),
            dias: por_estudiante.remove(&estudiante.id).unwrap_or_default(),
            estudiante,
        })
        .collect();

    let dias = fechas::dias_visibles(lunes, |d| {
        filas.iter().any(|f| !f.dias[d].productos.is_empty())
    });
    Ok(vista(
        lunes,
        id_grado,
        &dias,
        &filas,
        mensaje_flash(filtro.ok.as_deref()),
    ))
}

fn agregar<'a>(dias: &mut [Dia<'a>; 7], linea: &'a LineaSemana) {
    let dia = &mut dias[fechas::indice_dia(linea.fecha)];
    dia.total += linea.total_linea;
    match dia
        .productos
        .iter_mut()
        .find(|(nombre, _)| *nombre == linea.nombre_producto)
    {
        Some((_, cantidad)) => *cantidad += linea.cantidad,
        None => dia.productos.push((&linea.nombre_producto, linea.cantidad)),
    }
}

fn suma<'a>(filas: &'a [Fila<'a>], valor: impl Fn(&'a Fila<'a>) -> f64) -> f64 {
    filas.iter().map(valor).sum()
}

fn cabecera(lunes: NaiveDate, id_grado: i64) -> Markup {
    let (desde, hasta) = fechas::rango_semana(lunes);
    let herramienta = |href: &str, ico: Icono, etiqueta: &str| {
        html! {
            a href=(href) class="flex items-center gap-2 rounded-[0.9rem] px-4 py-2.5 text-[11px] font-bold text-gray-500 transition-all hover:bg-ios-bg hover:text-blue-600" {
                (icono(ico, "h-4 w-4")) span { (etiqueta) }
            }
        }
    };
    html! {
        header class="flex flex-col gap-6 p-2 py-4 lg:flex-row lg:items-center lg:justify-between" {
            div class="flex flex-1 flex-col items-center text-center lg:items-start lg:text-left" {
                h1 class="text-2xl font-extrabold tracking-tight text-gray-900 sm:text-3xl" { "Kiosco " span class="text-blue-600" { "del Valle" } }
                div class="mt-1 flex items-center gap-2 rounded-full border border-gray-200/70 bg-white px-4 py-1" {
                    p class="text-[13px] font-bold tracking-tight text-gray-500 uppercase" {
                        (desde) span class="mx-1 text-gray-300" { "—" } (hasta)
                    }
                }
            }
            div class="order-first flex flex-col items-center justify-center lg:order-none" {
                div class="inline-flex gap-2 rounded-2xl border border-gray-200/40 bg-gray-100/80 p-1 shadow-inner backdrop-blur-sm" {
                    a href=(rutas::semana(fechas::sumar_dias(lunes, -7), Some(id_grado))) title="Semana anterior"
                        class="rounded-xl p-2.5 text-gray-500 transition-all duration-200 hover:bg-white hover:text-blue-600 active:scale-90" {
                        (icono(Icono::ChevronIzquierda, "h-5 w-5"))
                    }
                    a href=(format!("/?grado={id_grado}")) class="rounded-xl bg-blue-600 px-8 py-2.5 text-sm font-bold text-white shadow-lg transition-all duration-200 hover:bg-blue-700" { "Semana actual" }
                    a href=(rutas::semana(fechas::sumar_dias(lunes, 7), Some(id_grado))) title="Semana siguiente"
                        class="rounded-xl p-2.5 text-gray-500 transition-all duration-200 hover:bg-white hover:text-blue-600 active:scale-90" {
                        (icono(Icono::ChevronDerecha, "h-5 w-5"))
                    }
                }
            }
            div class="hidden flex-1 items-center justify-end gap-3 lg:flex" {
                nav class="flex items-center gap-1 rounded-[1.25rem] border border-gray-200/40 bg-white p-1" {
                    (herramienta("/estudiantes", Icono::Estudiantes, "ALUMNOS"))
                    (herramienta("/productos", Icono::Productos, "PRODUCTOS"))
                    (herramienta("/cobranza", Icono::Dinero, "COBRANZA"))
                    (herramienta("/cierre", Icono::Portapapeles, "CIERRE"))
                    a href="/registro" class="flex items-center gap-2 rounded-[0.9rem] bg-blue-600 px-5 py-2.5 text-[11px] font-bold text-white shadow-lg shadow-blue-100 transition-all hover:bg-blue-700 active:scale-95" {
                        (icono(Icono::Registro, "h-4 w-4")) span { "REGISTRO" }
                    }
                }
            }
        }
    }
}

fn vista(
    lunes: NaiveDate,
    id_grado: i64,
    dias: &[NaiveDate],
    filas: &[Fila],
    flash: Option<&str>,
) -> Markup {
    let columnas = dias.len() + 5;

    layout::page(
        "Inicio",
        flash,
        html! {
            (bottom_nav())
            main class="mx-auto min-h-screen max-w-full bg-gray-50 p-4 pt-0 pb-32 sm:p-6 sm:pt-0" {
                (cabecera(lunes, id_grado))

                div class="sticky top-0 z-20 mb-4 rounded-xl border border-gray-200 bg-white" {
                    nav class="no-scrollbar -mb-px flex space-x-2 overflow-x-auto" aria-label="Grados" {
                        @for g in &GRADOS {
                            a href=(rutas::semana(lunes, Some(g.id)))
                                aria-current=[(g.id == id_grado).then_some("page")]
                                class=(if g.id == id_grado { "border-b-2 border-blue-500 px-4 py-3 text-sm font-medium whitespace-nowrap text-blue-600" } else { "border-b-2 border-transparent px-4 py-3 text-sm font-medium whitespace-nowrap text-gray-500 hover:border-gray-300 hover:text-gray-700" }) {
                                (g.nombre)
                            }
                        }
                    }
                }

                div class="rounded-xl border border-gray-200 bg-white shadow-sm" {
                    div class="overflow-x-auto" {
                        table class="tabla-consumo min-w-full divide-y divide-gray-200" {
                            thead {
                                tr class="bg-gray-50" {
                                    th scope="col" class="nombre-estudiante w-48 bg-white px-4 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase" { "Estudiante" }
                                    @for dia in dias {
                                        th scope="col" class="w-40 border-r border-gray-200 px-2 py-3 text-center text-xs font-medium tracking-wider text-gray-500 uppercase" { (fechas::corta(*dia)) }
                                    }
                                    @for etiqueta in ["Subtotal", "D Ante", "Pagos", "Total"] {
                                        th scope="col" class="w-20 px-2 py-3 text-right text-xs font-bold tracking-wider text-gray-500 uppercase" { (etiqueta) }
                                    }
                                }
                            }
                            tbody class="divide-y divide-gray-200 bg-white" {
                                @if filas.is_empty() {
                                    tr { td colspan=(columnas) class="px-4 py-12 text-center text-sm text-gray-500" { "No hay estudiantes activos en este grado." } }
                                }
                                @for (i, fila) in filas.iter().enumerate() {
                                    @let fondo = if i % 2 == 0 { "bg-gray-50" } else { "bg-white" };
                                    @let id = fila.estudiante.id;
                                    tr class=(fondo) {
                                        td class=(format!("nombre-estudiante w-48 px-4 py-3 text-sm font-medium text-gray-800 {fondo}")) {
                                            div class="flex items-center justify-between gap-2" {
                                                div class="flex-1 leading-tight break-words" { (fila.estudiante.nombre_completo()) }
                                                a href=(rutas::comprobante(id, lunes, Some(id_grado))) title="Ver comprobante semanal"
                                                    class="shrink-0 rounded-md p-1.5 text-blue-600 transition-colors hover:bg-blue-50 hover:text-blue-800" {
                                                    (icono(Icono::Recibo, "h-4 w-4"))
                                                }
                                            }
                                        }
                                        @for dia in dias {
                                            @let consumo = &fila.dias[fechas::indice_dia(*dia)];
                                            td class="p-0 text-sm text-gray-500 w-40 align-top border-r border-gray-200" {
                                                a href=(format!("/consumos/{id}?fecha={}&grado={id_grado}", *dia))
                                                    class="flex h-full min-h-[180px] flex-col rounded-sm p-1 transition-colors hover:bg-blue-50" {
                                                    div class="grid flex-grow grid-cols-3 content-start gap-1" {
                                                        @for (producto, cantidad) in &consumo.productos {
                                                            div class="flex h-fit flex-col items-center rounded bg-gray-50 p-0.5" {
                                                                span class="w-full text-center text-[9px] font-medium break-words text-gray-700" { (producto) }
                                                                span class="text-xs font-bold text-blue-900" { (cantidad) }
                                                            }
                                                        }
                                                    }
                                                    div class="mt-auto shrink-0 border-t border-gray-200 pt-1.5 text-center" {
                                                        span class="text-xs text-gray-500" { "Total:" }
                                                        span class=(if es_cero(consumo.total) { "ml-1 text-sm font-bold text-gray-300" } else { "ml-1 text-sm font-bold text-blue-700" }) {
                                                            (soles(consumo.total))
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        td class="w-20 bg-orange-50 px-2 py-4 text-right text-sm font-semibold whitespace-nowrap text-orange-700" { (soles(fila.subtotal())) }
                                        td class="w-20 bg-orange-50 px-2 py-4 text-right text-sm font-semibold whitespace-nowrap text-orange-700" { (soles(fila.deuda)) }
                                        td class="w-20 bg-amber-100 px-2 py-4 text-right text-sm font-semibold whitespace-nowrap text-red-800" {
                                            a href=(rutas::pagos(id, lunes, Some(id_grado))) class="block rounded px-2 py-1 transition-colors hover:bg-amber-200" {
                                                @if es_cero(fila.pagos) { "-" } @else { (soles(fila.pagos)) }
                                            }
                                        }
                                        td class=(if fila.total() > 0.005 { "w-20 bg-red-100 px-2 py-4 text-right text-sm font-extrabold whitespace-nowrap text-red-700" } else { "w-20 bg-green-50 px-2 py-4 text-right text-sm font-extrabold whitespace-nowrap text-green-700" }) {
                                            (soles(fila.total()))
                                        }
                                    }
                                }
                            }
                            @if !filas.is_empty() {
                                tfoot class="border-t-2 border-gray-300 bg-gray-100 text-sm font-bold text-gray-700" {
                                    tr {
                                        td class="nombre-estudiante bg-gray-100 px-4 py-3" { (filas.len()) " estudiantes" }
                                        @for dia in dias {
                                            @let d = fechas::indice_dia(*dia);
                                            td class="border-r border-gray-200 px-2 py-3 text-center tabular-nums" {
                                                "S/ " (monto(filas.iter().map(|f| f.dias[d].total).sum()))
                                            }
                                        }
                                        td class="px-2 py-3 text-right whitespace-nowrap" { (soles(suma(filas, Fila::subtotal))) }
                                        td class="px-2 py-3 text-right whitespace-nowrap" { (soles(suma(filas, |f| f.deuda))) }
                                        td class="px-2 py-3 text-right whitespace-nowrap" { (soles(suma(filas, |f| f.pagos))) }
                                        td class="px-2 py-3 text-right whitespace-nowrap" { (soles(suma(filas, Fila::total))) }
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
