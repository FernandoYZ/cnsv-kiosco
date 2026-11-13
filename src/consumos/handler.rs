use std::collections::{BTreeMap, HashMap};

use axum::{
    Form,
    extract::{Path, Query, State},
    response::Redirect,
};
use chrono::NaiveDate;
use maud::{Markup, html};
use serde::Deserialize;
use tokio_rusqlite::rusqlite;

use super::{repository, repository::LineaResumen, service};
use crate::{
    error::AppError,
    estudiantes::{self, Estudiante, Sector, grado_desde},
    fechas, forms,
    productos::{self, Producto},
    rutas,
    state::AppState,
    templates::{
        components::{
            Icono, avatar, estado_vacio, icono, ios_nav, mensaje_flash, monto, selector_dia,
        },
        layout,
    },
};

#[derive(Deserialize)]
pub struct DiaQuery {
    fecha: Option<String>,
    ok: Option<String>,
}

#[derive(Deserialize)]
pub struct EditorQuery {
    fecha: Option<String>,
    sector: Option<String>,
    grado: Option<String>,
}

fn sector_de_ruta(clave: &str) -> Result<Sector, AppError> {
    Sector::desde(clave).ok_or(AppError::NotFound)
}

fn url_sector(sector: Sector, fecha: NaiveDate) -> String {
    format!("/registro/{}?fecha={}", sector.clave(), fecha)
}

/// Where the editor returns: the sector list or the weekly table.
fn url_volver(sector: Option<Sector>, grado: Option<i64>, fecha: NaiveDate) -> String {
    match (sector, grado) {
        (Some(sector), _) => url_sector(sector, fecha),
        (None, grado) => rutas::semana(fecha, grado),
    }
}

/// `/registro`: choose the work area.
pub async fn inicio() -> Markup {
    let tarjeta = |href: &str,
                   ico: Icono,
                   color: &str,
                   titulo: &str,
                   texto: Markup,
                   accion: bool| {
        html! {
            a href=(href) class="group flex h-64 flex-col justify-between rounded-[24px] border border-gray-200/60 bg-white p-8 text-left shadow-sm transition-all duration-200 hover:shadow-md active:scale-95" {
                div class=(format!("flex h-14 w-14 items-center justify-center rounded-2xl transition-colors {color}")) {
                    (icono(ico, "h-8 w-8"))
                }
                div {
                    h2 class="text-[22px] leading-tight font-extrabold text-gray-900" { (titulo) }
                    p class="mt-2 text-[15px] font-semibold text-ios-gray" { (texto) }
                }
                @if accion { span class="text-sm font-semibold text-ios-blue" { "Seleccionar →" } }
            }
        }
    };
    layout::page(
        "Registro de Consumos",
        None,
        html! {
            nav class="ios-nav" {
                div class="mx-auto flex max-w-5xl items-center justify-between" {
                    a href="/" class="text-[15px] font-bold tracking-tight text-gray-900" { "Kiosco " span class="text-ios-blue" { "del Valle" } }
                    div class="flex items-center gap-4" {
                        a href="/" class="ios-link text-[15px] font-medium" {
                            span { "Inicio" }
                            (icono(Icono::Inicio, "ml-2 h-5 w-5"))
                        }
                        a href="/logout" class="inline-flex items-center gap-1.5 text-[15px] font-medium text-ios-gray" {
                            (icono(Icono::Salir, "h-5 w-5")) span { "Salir" }
                        }
                    }
                }
            }
            main class="mx-auto max-w-5xl px-6 pt-12 pb-10" {
                header class="mb-12" {
                    h1 class="text-[34px] leading-tight font-extrabold tracking-tight text-gray-900" { "Registro de Consumos" }
                    p class="mt-2 text-[19px] font-medium text-ios-gray" { "Selecciona el área de trabajo para hoy" }
                }
                div class="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3" {
                    (tarjeta("/registro/menor", Icono::Estudiantes, "bg-indigo-50 text-indigo-600 group-active:bg-indigo-600 group-active:text-white", "Sector Menor", html! { "Primaria y Bajos" br; "Secundaria" }, true))
                    (tarjeta("/registro/mayor", Icono::Estudiantes, "bg-blue-50 text-blue-600 group-active:bg-blue-600 group-active:text-white", "Sector Mayor", html! { "3ro a 5to de" br; "Secundaria" }, true))
                    (tarjeta("/productos", Icono::Productos, "bg-orange-50 text-orange-600 group-active:bg-orange-600 group-active:text-white", "Productos", html! { "Configurar precios" br; "y existencias" }, false))
                }
            }
        },
    )
}

/// `/registro/{sector}`: searchable student list for one day.
pub async fn sector(
    State(state): State<AppState>,
    Path(clave): Path<String>,
    Query(q): Query<DiaQuery>,
) -> Result<Markup, AppError> {
    let sector = sector_de_ruta(&clave)?;
    let fecha = fechas::parse_o_hoy(q.fecha.as_deref());
    let (estudiantes, items) = state
        .db
        .call(move |c| {
            Ok::<_, rusqlite::Error>((
                estudiantes::repository::activos_por_sector(c, sector)?,
                repository::items_por_estudiante(c, fecha)?,
            ))
        })
        .await?;
    Ok(vista_sector(
        sector,
        fecha,
        &estudiantes,
        &items,
        mensaje_flash(q.ok.as_deref()),
    ))
}

fn vista_sector(
    sector: Sector,
    fecha: NaiveDate,
    estudiantes: &[Estudiante],
    items: &HashMap<i64, i64>,
    flash: Option<&str>,
) -> Markup {
    let resumen = format!("/resumen/{}?fecha={}", sector.clave(), fecha);
    let con_consumo = estudiantes
        .iter()
        .filter(|e| items.contains_key(&e.id))
        .count();
    layout::page(
        sector.nombre(),
        flash,
        html! {
            (ios_nav(("/registro", "Atrás"), sector.nombre(), html! {
                a href=(resumen) class="ios-link text-[15px] font-medium" { "Resumen" }
            }))
            (selector_dia(fecha, |d| url_sector(sector, d)))
            div class="mx-auto mb-4 max-w-2xl px-3 pt-4 sm:mb-6 sm:px-4 sm:pt-6 lg:max-w-6xl" {
                h1 class="text-2xl leading-tight font-bold tracking-tight text-gray-900 sm:text-3xl lg:text-[34px]" { "Consumo" }
                p class="mt-1 text-[15px] font-medium text-ios-gray sm:mt-2 sm:text-[17px]" {
                    "Selecciona un estudiante para registrar · " (con_consumo) " de " (estudiantes.len()) " con consumo"
                }
            }
            div data-student-filter class="mx-auto max-w-2xl px-3 pb-10 sm:px-4 lg:max-w-6xl" {
                div class="items-start md:grid md:grid-cols-12 md:gap-6 lg:gap-8" {
                    aside class="mb-6 md:sticky md:top-20 md:col-span-5 md:mb-0 lg:col-span-4" {
                        h3 class="section-title" { "Búsqueda" }
                        div class="ios-card rounded-2xl sm:rounded-[28px]" {
                            div class="divide-y divide-gray-100" {
                                label class="flex items-center gap-3 px-4 py-4" {
                                    (icono(Icono::Buscar, "h-5 w-5 text-gray-400"))
                                    input data-filter-search type="search" placeholder="Nombre..." aria-label="Buscar estudiante"
                                        class="flex-1 border-none bg-transparent text-[17px] placeholder-gray-400";
                                }
                                div class="bg-gray-50/50 p-3 sm:p-4" {
                                    div class="no-scrollbar flex gap-2 overflow-x-auto pb-1" {
                                        button type="button" data-filter-grade="" class="pill pill-active" { "Todos" }
                                        @for g in sector.grados() {
                                            button type="button" data-filter-grade=(g.nombre) class="pill" { (g.nombre) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    section class="md:col-span-7 lg:col-span-8" {
                        div class="mb-3 flex items-center justify-between px-4" {
                            h3 class="text-[13px] font-bold tracking-wide text-ios-gray uppercase" { "Listado" }
                            span data-filter-count class="badge bg-blue-50 text-ios-blue" { "encontrados: " (estudiantes.len()) }
                        }
                        div class="ios-card rounded-2xl sm:rounded-[28px]" {
                            div class="divide-y divide-gray-100" {
                                @for e in estudiantes {
                                    @let cantidad = items.get(&e.id).copied().unwrap_or(0);
                                    a data-row data-name=(format!("{} {}", e.apellidos, e.nombres)) data-grade=(e.grado())
                                        href=(format!("/consumos/{}?fecha={}&sector={}", e.id, fecha, sector.clave()))
                                        class="flex items-center justify-between gap-3 p-3 transition-colors select-none hover:bg-gray-50 active:bg-gray-100 sm:p-4" {
                                        div class="flex min-w-0 items-center gap-4" {
                                            (avatar(&e.apellidos, true))
                                            div class="min-w-0" {
                                                p class="truncate text-[15px] leading-tight font-bold text-gray-900 sm:text-[17px]" { (e.nombre_completo()) }
                                                p class="text-[13px] font-medium text-ios-gray sm:text-[15px]" { (e.grado()) }
                                            }
                                        }
                                        div class="flex shrink-0 items-center gap-2" {
                                            @if cantidad > 0 {
                                                span class="badge bg-ios-green-soft text-[12px] text-green-700" { (cantidad) " ítem(s)" }
                                            }
                                            (icono(Icono::ChevronDerecha, "h-5 w-5 text-gray-300"))
                                        }
                                    }
                                }
                            }
                            div data-filter-empty class=(if estudiantes.is_empty() { "" } else { "hidden" }) {
                                (estado_vacio(Icono::Persona, "Sin resultados", "No encontramos estudiantes con esos criterios."))
                            }
                        }
                    }
                }
            }
        },
    )
}

/// `/consumos/{id}`: what one student consumed on one day.
pub async fn editor(
    State(state): State<AppState>,
    Path(id_estudiante): Path<i64>,
    Query(q): Query<EditorQuery>,
) -> Result<Markup, AppError> {
    let fecha = fechas::parse_o_hoy(q.fecha.as_deref());
    let sector = q.sector.as_deref().and_then(Sector::desde);
    let (estudiante, productos, cantidades) = state
        .db
        .call(move |c| {
            Ok::<_, rusqlite::Error>((
                estudiantes::repository::por_id(c, id_estudiante)?,
                productos::repository::activos(c)?,
                repository::cantidades_de_estudiante(c, id_estudiante, fecha)?,
            ))
        })
        .await?;
    let estudiante = estudiante.ok_or(AppError::NotFound)?;
    let grado = grado_desde(q.grado.as_deref()).or(estudiante.id_grado);
    Ok(vista_editor(
        &estudiante,
        fecha,
        sector,
        grado,
        &productos,
        &cantidades,
    ))
}

/// Fields: `fecha`, optional `sector` / `grado` (return target) and `cantidad_{id_producto}`.
pub async fn guardar(
    State(state): State<AppState>,
    Path(id_estudiante): Path<i64>,
    Form(campos): Form<Vec<(String, String)>>,
) -> Result<Redirect, AppError> {
    let mut fecha = None;
    let mut sector = None;
    let mut grado = None;
    let mut cantidades = Vec::new();
    for (nombre, valor) in &campos {
        match nombre.as_str() {
            "fecha" => fecha = Some(forms::fecha(valor, "Fecha")?),
            "sector" => sector = Sector::desde(valor),
            "grado" => grado = grado_desde(Some(valor)),
            _ => {
                if let Some(id_producto) = nombre
                    .strip_prefix("cantidad_")
                    .and_then(|id| id.parse::<i64>().ok())
                {
                    cantidades.push((id_producto, forms::cantidad(valor)?));
                }
            }
        }
    }
    let fecha = fecha.ok_or_else(|| AppError::BadRequest("Falta la fecha.".to_owned()))?;

    state
        .db
        .call(move |c| service::guardar_dia(c, id_estudiante, fecha, &cantidades))
        .await?
        .ok_or(AppError::NotFound)?;

    let destino = url_volver(sector, grado, fecha);
    Ok(Redirect::to(&format!("{destino}&ok=consumo")))
}

fn vista_editor(
    estudiante: &Estudiante,
    fecha: NaiveDate,
    sector: Option<Sector>,
    grado: Option<i64>,
    productos: &[Producto],
    cantidades: &HashMap<i64, i64>,
) -> Markup {
    let volver = url_volver(sector, grado, fecha);
    let etiqueta_volver = match sector {
        Some(s) => format!("Volver a {}", s.nombre()),
        None => "Volver a la semana".to_owned(),
    };
    layout::page(
        &format!("Editar Consumo - {}", estudiante.nombre_completo()),
        None,
        html! {
            main class="mx-auto max-w-2xl px-4 pt-4 pb-56 lg:max-w-6xl lg:pt-8 lg:pb-10" {
                div class="mb-6" {
                    a href=(volver) class="mb-2 inline-flex items-center gap-1 font-medium text-ios-blue transition-colors hover:text-blue-700" {
                        (icono(Icono::ChevronIzquierda, "h-5 w-5")) (etiqueta_volver)
                    }
                    h1 class="text-2xl font-extrabold tracking-tight text-gray-900 lg:text-3xl" { (estudiante.nombre_completo()) }
                    p class="text-sm text-gray-500 lg:text-base" { (fechas::larga(fecha)) " · " (estudiante.grado()) }
                }
                @if !estudiante.esta_activo {
                    p class="ios-card mb-4 p-4 text-[15px] font-semibold text-ios-red" { "Este estudiante está dado de baja: actívalo para registrar consumos." }
                }
                form method="post" action=(format!("/consumos/{}", estudiante.id)) data-consumo-editor
                    class="lg:grid lg:grid-cols-12 lg:items-start lg:gap-8" {
                    input type="hidden" name="fecha" value=(fecha);
                    @if let Some(s) = sector { input type="hidden" name="sector" value=(s.clave()); }
                    @if let Some(g) = grado { input type="hidden" name="grado" value=(g); }

                    div class="lg:col-span-7 xl:col-span-8" {
                        div class="flex flex-col overflow-hidden rounded-2xl border border-gray-200 bg-white shadow-sm" {
                            div class="hidden items-center justify-between border-b border-gray-200/70 bg-gray-50 px-4 py-3 lg:flex" {
                                span class="text-xs font-bold tracking-wider text-gray-400 uppercase" { "Productos disponibles" }
                                span class="text-[11px] text-gray-400" { (productos.len()) " productos" }
                            }
                            @if productos.is_empty() {
                                (estado_vacio(Icono::Productos, "Sin productos activos", "Activa productos en el catálogo para registrar consumos."))
                            }
                            @for p in productos {
                                @let cantidad = cantidades.get(&p.id).copied().unwrap_or(0);
                                div data-product data-price=(p.precio_unitario)
                                    class="flex items-center justify-between gap-4 border-b border-gray-200/70 px-4 py-4 transition-colors last:border-b-0 hover:bg-gray-50/50" {
                                    div class="min-w-0 flex-1" {
                                        p class="truncate font-bold text-gray-900 lg:text-lg" { (p.nombre) }
                                        p class="text-sm font-medium text-gray-500" { "S/ " (monto(p.precio_unitario)) }
                                    }
                                    div class="flex items-center gap-1 rounded-xl bg-gray-100 p-1" {
                                        button type="button" data-step="-1" aria-label=(format!("Quitar {}", p.nombre))
                                            class="flex h-10 w-10 items-center justify-center rounded-lg bg-white text-xl font-bold text-gray-700 shadow-sm transition-all active:scale-90" { "−" }
                                        input type="number" data-qty name=(format!("cantidad_{}", p.id)) value=(cantidad)
                                            min="0" max="999" inputmode="numeric" aria-label=(format!("Cantidad de {}", p.nombre))
                                            class="w-12 border-0 bg-transparent text-center text-lg font-bold text-gray-900";
                                        button type="button" data-step="1" aria-label=(format!("Agregar {}", p.nombre))
                                            class="flex h-10 w-10 items-center justify-center rounded-lg bg-white text-xl font-bold text-gray-700 shadow-sm transition-all active:scale-90" { "+" }
                                    }
                                }
                            }
                        }
                    }

                    div class="fixed right-0 bottom-0 left-0 border-t border-gray-200 bg-white/80 p-4 pb-8 backdrop-blur-xl lg:sticky lg:top-8 lg:col-span-5 lg:border-0 lg:bg-transparent lg:p-0 lg:backdrop-blur-none xl:col-span-4" {
                        div class="mx-auto max-w-2xl lg:rounded-2xl lg:border lg:border-gray-200 lg:bg-white lg:p-6 lg:shadow-sm" {
                            div class="mb-4 flex items-center justify-between px-2 lg:mb-6 lg:px-0" {
                                div class="flex flex-col" {
                                    span class="text-[10px] font-semibold tracking-wider text-gray-500 uppercase lg:text-xs" { "Total del día" }
                                    span class="hidden text-[10px] text-gray-400 lg:block" { "Calculado con el precio vigente" }
                                }
                                span class="text-2xl font-black text-gray-900 tabular-nums lg:text-3xl" { "S/ " span data-total { "0.00" } }
                            }
                            div class="flex flex-col gap-3" {
                                button type="submit" class="btn-ios" disabled[!estudiante.esta_activo || productos.is_empty()] { "Guardar Cambios" }
                                a href=(volver) class="w-full py-2 text-center text-sm font-medium text-gray-400 transition-colors hover:text-gray-600" { "Cancelar y Volver" }
                            }
                        }
                    }
                }
            }
        },
    )
}

/// `/resumen/{sector}`: what to deliver, per student, for one day.
pub async fn resumen(
    State(state): State<AppState>,
    Path(clave): Path<String>,
    Query(q): Query<DiaQuery>,
) -> Result<Markup, AppError> {
    let sector = sector_de_ruta(&clave)?;
    let fecha = fechas::parse_o_hoy(q.fecha.as_deref());
    let lineas = state
        .db
        .call(move |c| repository::resumen_diario(c, sector, fecha))
        .await?;
    Ok(vista_resumen(sector, fecha, &lineas))
}

fn vista_resumen(sector: Sector, fecha: NaiveDate, lineas: &[LineaResumen]) -> Markup {
    // Rows arrive ordered by student, so consecutive lines belong to the same student.
    let por_estudiante: Vec<&[LineaResumen]> = lineas
        .chunk_by(|a, b| a.id_estudiante == b.id_estudiante)
        .collect();
    let mut por_producto: BTreeMap<&str, i64> = BTreeMap::new();
    for linea in lineas {
        *por_producto
            .entry(linea.nombre_producto.as_str())
            .or_default() += linea.cantidad;
    }
    let total_items: i64 = por_producto.values().sum();

    layout::page(
        "Resumen del Día",
        None,
        html! {
            (ios_nav((&url_sector(sector, fecha), "Atrás"), sector.nombre(), html! {
                span class=(if total_items > 0 { "badge bg-ios-green px-2.5 py-1 font-bold text-white" } else { "badge bg-ios-gray px-2.5 py-1 font-bold text-white" }) {
                    (total_items)
                }
            }))
            (selector_dia(fecha, |d| format!("/resumen/{}?fecha={}", sector.clave(), d)))
            div class="mx-auto mb-4 flex max-w-2xl items-end justify-between gap-4 px-3 pt-2 sm:px-4 lg:max-w-6xl" {
                div {
                    h1 class="text-2xl leading-tight font-bold tracking-tight text-gray-900 sm:text-3xl lg:text-[34px]" { "Resumen del Día" }
                    p class="mt-1 text-[15px] font-medium text-ios-gray sm:text-[17px]" {
                        @if por_estudiante.is_empty() { "Sin consumos registrados" }
                        @else { (por_estudiante.len()) " estudiante(s) con consumo · " (fechas::larga(fecha)) }
                    }
                }
                @if !por_estudiante.is_empty() {
                    button type="button" onclick="window.print()" class="btn-text shrink-0 bg-white text-ios-blue shadow-sm print:hidden" { "Imprimir" }
                }
            }
            main class="mx-auto max-w-2xl space-y-3 px-3 pb-10 sm:px-4 lg:max-w-6xl" {
                @if por_estudiante.is_empty() {
                    div class="ios-card rounded-[28px]" {
                        (estado_vacio(Icono::Portapapeles, "Sin consumos", "No hay consumos registrados para este sector en la fecha seleccionada."))
                    }
                } @else {
                    section class="ios-card rounded-[28px] p-4" {
                        h2 class="mb-3 text-[13px] font-bold tracking-wide text-ios-gray uppercase" { "Total por producto" }
                        div class="flex flex-wrap gap-2" {
                            @for (producto, cantidad) in &por_producto {
                                span class="inline-flex items-center rounded-xl border border-blue-100 bg-blue-50 px-3 py-1.5 text-[13px] font-semibold text-gray-800" {
                                    (producto) span class="ml-1 font-bold text-ios-blue" { "×" (cantidad) }
                                }
                            }
                        }
                    }
                    div class="grid gap-3 lg:grid-cols-2" {
                        @for &grupo in &por_estudiante {
                            @let e = &grupo[0];
                            article class="ios-card rounded-2xl break-inside-avoid sm:rounded-[28px]" {
                                div class="flex items-center gap-4 border-b border-gray-100 px-4 py-3" {
                                    (avatar(&e.apellidos, true))
                                    div class="min-w-0 flex-1" {
                                        p class="truncate text-[15px] leading-tight font-bold text-gray-900 sm:text-[17px]" { (e.apellidos) ", " (e.nombres) }
                                        p class="text-[13px] font-medium text-ios-gray sm:text-[15px]" { (e.nombre_grado) }
                                    }
                                    span class="badge shrink-0 bg-blue-50 text-ios-blue" { (grupo.len()) " ítem(s)" }
                                }
                                div class="flex flex-wrap gap-2 px-4 pt-3 pb-4" {
                                    @for linea in grupo {
                                        span class="inline-flex items-center rounded-xl border border-gray-100 bg-ios-bg px-3 py-1.5 text-[13px] font-medium text-gray-800" {
                                            (linea.nombre_producto)
                                            @if linea.cantidad > 1 { span class="ml-1 font-bold text-ios-blue" { "×" (linea.cantidad) } }
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
