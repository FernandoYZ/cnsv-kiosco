use std::collections::{BTreeMap, HashMap};

use axum::{
    Form,
    extract::{Query, State},
    response::Redirect,
};
use chrono::NaiveDate;
use maud::{Markup, html};
use serde::Deserialize;
use tokio_rusqlite::rusqlite;

use super::{
    repository::{self, LineaResumen},
    service::{self, Celda},
};
use crate::{
    error::AppError,
    estudiantes::{self, Estudiante, Sector},
    fechas, forms,
    productos::{self, Producto},
    state::AppState,
    templates::{
        components::{alerta_ok, page_header},
        layout,
    },
};

#[derive(Deserialize)]
pub struct DiaQuery {
    sector: Option<String>,
    fecha: Option<String>,
    guardados: Option<usize>,
}

pub async fn registro(
    State(state): State<AppState>,
    Query(q): Query<DiaQuery>,
) -> Result<Markup, AppError> {
    let sector = Sector::desde(q.sector.as_deref());
    let fecha = fechas::parse_o_hoy(q.fecha.as_deref());
    let (estudiantes, productos, cantidades) = state
        .db
        .call(move |c| {
            Ok::<_, rusqlite::Error>((
                estudiantes::repository::activos_por_sector(c, sector)?,
                productos::repository::activos(c)?,
                repository::cantidades_del_dia(c, fecha)?,
            ))
        })
        .await?;
    Ok(vista_registro(
        sector,
        fecha,
        &estudiantes,
        &productos,
        &cantidades,
        q.guardados,
    ))
}

/// Grid inputs are named `c-{id_estudiante}-{id_producto}`.
pub async fn guardar_registro(
    State(state): State<AppState>,
    Form(campos): Form<Vec<(String, String)>>,
) -> Result<Redirect, AppError> {
    let mut sector = None;
    let mut fecha = None;
    let mut celdas = Vec::new();

    for (nombre, valor) in &campos {
        match nombre.as_str() {
            "sector" => sector = Some(Sector::desde(Some(valor))),
            "fecha" => fecha = Some(forms::fecha(valor, "Fecha")?),
            _ => {
                if let Some((id_estudiante, id_producto)) = parse_celda(nombre) {
                    celdas.push(Celda {
                        id_estudiante,
                        id_producto,
                        cantidad: forms::cantidad(valor)?,
                    });
                }
            }
        }
    }

    let sector = sector.ok_or_else(|| AppError::BadRequest("Falta el sector.".to_owned()))?;
    let fecha = fecha.ok_or_else(|| AppError::BadRequest("Falta la fecha.".to_owned()))?;

    let resultado = state
        .db
        .call(move |c| service::guardar_registro(c, sector, fecha, &celdas))
        .await?;
    if resultado.ignoradas > 0 {
        tracing::warn!(
            ignoradas = resultado.ignoradas,
            "grid cells ignored (inactive product or student outside sector)"
        );
    }

    Ok(Redirect::to(&format!(
        "/registro?sector={}&fecha={}&guardados={}",
        sector.clave(),
        fechas::iso(fecha),
        resultado.cambios
    )))
}

fn parse_celda(nombre: &str) -> Option<(i64, i64)> {
    let (estudiante, producto) = nombre.strip_prefix("c-")?.split_once('-')?;
    Some((estudiante.parse().ok()?, producto.parse().ok()?))
}

pub async fn resumen(
    State(state): State<AppState>,
    Query(q): Query<DiaQuery>,
) -> Result<Markup, AppError> {
    let sector = Sector::desde(q.sector.as_deref());
    let fecha = fechas::parse_o_hoy(q.fecha.as_deref());
    let lineas = state
        .db
        .call(move |c| repository::resumen_diario(c, sector, fecha))
        .await?;
    Ok(vista_resumen(sector, fecha, &lineas))
}

/// Sector tabs plus previous/next day navigation for `ruta`.
fn selector_dia(ruta: &str, sector: Sector, fecha: NaiveDate) -> Markup {
    let enlace =
        |s: Sector, f: NaiveDate| format!("{ruta}?sector={}&fecha={}", s.clave(), fechas::iso(f));
    html! {
        div class="mb-4 flex flex-wrap items-center gap-3 print:hidden" {
            div class="flex gap-1" {
                @for s in Sector::TODOS {
                    a href=(enlace(s, fecha))
                        class=(if s == sector { "btn-primary" } else { "btn-secondary" }) { (s.nombre()) }
                }
            }
            div class="flex items-center gap-1" {
                a class="btn-secondary" href=(enlace(sector, fechas::sumar_dias(fecha, -1))) { "← Anterior" }
                form method="get" action=(ruta) class="flex items-center gap-1" {
                    input type="hidden" name="sector" value=(sector.clave());
                    input class="input" type="date" name="fecha" value=(fechas::iso(fecha)) onchange="this.form.submit()";
                }
                a class="btn-secondary" href=(enlace(sector, fechas::sumar_dias(fecha, 1))) { "Siguiente →" }
                a class="btn-link ml-2" href=(enlace(sector, fechas::hoy())) { "Hoy" }
            }
        }
    }
}

fn vista_registro(
    sector: Sector,
    fecha: NaiveDate,
    estudiantes: &[Estudiante],
    productos: &[Producto],
    cantidades: &HashMap<(i64, i64), i64>,
    guardados: Option<usize>,
) -> Markup {
    layout::page(
        "Registro diario",
        "registro",
        html! {
            (page_header("Registro diario", Some(&format!("{} · {}", sector.nombre(), fechas::larga(fecha)))))
            @if let Some(n) = guardados {
                (alerta_ok(&format!("Registro guardado: {n} celda(s) modificada(s).")))
            }
            (selector_dia("/registro", sector, fecha))
            @if estudiantes.is_empty() || productos.is_empty() {
                p class="card text-slate-500" { "No hay estudiantes o productos activos para este sector." }
            } @else {
                form method="post" action="/registro" {
                    input type="hidden" name="sector" value=(sector.clave());
                    input type="hidden" name="fecha" value=(fechas::iso(fecha));
                    div class="card max-h-[70vh] overflow-auto p-0" {
                        table class="table-base" {
                            thead class="sticky top-0 z-10" {
                                tr {
                                    th class="sticky left-0 z-20" { "Estudiante" }
                                    @for p in productos {
                                        th class="text-center" {
                                            (p.nombre)
                                            span class="block text-xs font-normal text-slate-400" { (format!("{:.2}", p.precio_unitario)) }
                                        }
                                    }
                                }
                            }
                            tbody {
                                @for e in estudiantes {
                                    tr class="hover:bg-emerald-50/40" {
                                        td class="sticky left-0 bg-white whitespace-nowrap" {
                                            (e.nombre_completo())
                                            span class="block text-xs text-slate-400" { (e.grado()) }
                                        }
                                        @for p in productos {
                                            @let cantidad = cantidades.get(&(e.id, p.id)).copied().unwrap_or(0);
                                            td class="px-1 text-center" {
                                                input class=(if cantidad > 0 { "input w-14 bg-emerald-50 text-center font-semibold" } else { "input w-14 text-center" })
                                                    type="number" min="0" max="999" inputmode="numeric" placeholder="0"
                                                    aria-label=(format!("{} – {}", e.nombre_completo(), p.nombre))
                                                    name=(format!("c-{}-{}", e.id, p.id))
                                                    value=[(cantidad > 0).then_some(cantidad)];
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div class="mt-4 flex justify-end" {
                        button type="submit" class="btn-primary" { "Guardar registro" }
                    }
                }
            }
        },
    )
}

fn vista_resumen(sector: Sector, fecha: NaiveDate, lineas: &[LineaResumen]) -> Markup {
    // Rows arrive ordered by student, so consecutive lines belong to the same student.
    let mut por_estudiante: Vec<(&LineaResumen, Vec<&LineaResumen>)> = Vec::new();
    for linea in lineas {
        match por_estudiante.last_mut() {
            Some((primera, grupo)) if primera.id_estudiante == linea.id_estudiante => {
                grupo.push(linea)
            }
            _ => por_estudiante.push((linea, vec![linea])),
        }
    }
    let mut por_producto: BTreeMap<&str, i64> = BTreeMap::new();
    for linea in lineas {
        *por_producto
            .entry(linea.nombre_producto.as_str())
            .or_default() += linea.cantidad;
    }

    layout::page(
        "Resumen del día",
        "resumen",
        html! {
            (page_header("Resumen del día", Some(&format!("{} · {}", sector.nombre(), fechas::larga(fecha)))))
            (selector_dia("/resumen", sector, fecha))
            @if lineas.is_empty() {
                p class="card text-slate-500" { "No hay consumos registrados para este día." }
            } @else {
                div class="mb-4 flex justify-end print:hidden" {
                    button type="button" class="btn-secondary" onclick="window.print()" { "Imprimir" }
                }
                section class="card mb-6" {
                    h2 class="mb-2 font-semibold" { "Total por producto" }
                    ul class="flex flex-wrap gap-2" {
                        @for (producto, cantidad) in &por_producto {
                            li class="rounded-md bg-slate-100 px-2 py-1 text-sm" {
                                span class="font-semibold tabular-nums" { (cantidad) } " × " (producto)
                            }
                        }
                    }
                }
                div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3" {
                    @for (estudiante, grupo) in &por_estudiante {
                        article class="card break-inside-avoid" {
                            h3 class="font-medium" { (estudiante.apellidos) ", " (estudiante.nombres) }
                            p class="mb-2 text-xs text-slate-500" { (estudiante.nombre_grado) }
                            ul class="text-sm" {
                                @for linea in grupo {
                                    li { span class="font-semibold tabular-nums" { (linea.cantidad) } " × " (linea.nombre_producto) }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}
