use axum::extract::{Query, State};
use maud::{Markup, html};
use serde::Deserialize;

use super::repository::{self, Saldo};
use crate::{
    error::AppError,
    estudiantes::{GRADOS, grado_desde},
    fechas, rutas,
    state::AppState,
    templates::{
        components::{Icono, avatar, estado_vacio, icono, ios_nav, monto, soles},
        layout,
    },
};

#[derive(Deserialize)]
pub struct Filtro {
    grado: Option<String>,
}

/// Message shared with a family (WhatsApp, SMS…) from the collections list.
fn mensaje(s: &Saldo) -> String {
    format!(
        "Hola, le saludamos del Kiosco del Valle. {} {} ({}) tiene un saldo pendiente de S/ {}. ¡Gracias por su preferencia!",
        s.estudiante.nombres,
        s.estudiante.apellidos,
        s.estudiante.grado(),
        monto(s.saldo)
    )
}

/// `/cobranza`: students with debt, highest first.
pub async fn index(
    State(state): State<AppState>,
    Query(filtro): Query<Filtro>,
) -> Result<Markup, AppError> {
    let id_grado = grado_desde(filtro.grado.as_deref());
    let saldos = state
        .db
        .call(move |c| repository::saldos(c, id_grado.unwrap_or(0)))
        .await?;
    let deudores: Vec<&Saldo> = saldos.iter().filter(|s| s.saldo > 0.005).collect();
    let al_dia = saldos.len() - deudores.len();
    Ok(vista(&deudores, al_dia, id_grado))
}

fn vista(deudores: &[&Saldo], al_dia: usize, id_grado: Option<i64>) -> Markup {
    let total: f64 = deudores.iter().map(|s| s.saldo).sum();
    let hoy = fechas::hoy();
    let pill = |href: String, etiqueta: &str, activo: bool| {
        html! { a href=(href) data-replace class=(if activo { "pill pill-active" } else { "pill" }) { (etiqueta) } }
    };

    layout::page(
        "Cobranza",
        None,
        html! {
            (ios_nav(("/", "Atrás"), "Cobranza", html! {}))
            main class="mx-auto max-w-2xl px-4 pt-4 pb-10 sm:pt-6 lg:max-w-4xl" {
                header class="mb-4 sm:mb-6" {
                    h1 class="text-[28px] leading-tight font-bold tracking-tight text-gray-900 sm:text-[34px]" { "Cobranza" }
                }
                div class="ios-card mb-4 p-4 sm:mb-6 sm:p-5" {
                    p class="text-[13px] font-semibold tracking-wide text-ios-gray uppercase" { "Total por cobrar" }
                    p class="mt-1 text-[32px] leading-none font-black text-ios-red tabular-nums sm:text-[36px]" { (soles(total)) }
                    div class="mt-3 flex flex-wrap gap-2" {
                        span class="badge inline-flex items-center gap-1.5 bg-gray-100 text-gray-700" { span class="h-1.5 w-1.5 rounded-full bg-ios-red" {} (deudores.len()) " con deuda" }
                        span class="badge inline-flex items-center gap-1.5 bg-gray-100 text-gray-700" { span class="h-1.5 w-1.5 rounded-full bg-ios-green" {} (al_dia) " al día" }
                    }
                }
                nav class="no-scrollbar -mx-4 mb-4 flex gap-2 overflow-x-auto px-4 pb-1 sm:mx-0 sm:mb-6 sm:px-1" aria-label="Filtrar por grado" {
                    (pill("/cobranza".to_owned(), "Todos", id_grado.is_none()))
                    @for g in &GRADOS {
                        (pill(format!("/cobranza?grado={}", g.id), g.nombre, id_grado == Some(g.id)))
                    }
                }
                div class="ios-card" {
                    @if deudores.is_empty() {
                        (estado_vacio(Icono::Check, "Todo al día", "No hay estudiantes con saldo pendiente."))
                    } @else {
                        div class="divide-y divide-gray-100" {
                            @for s in deudores {
                                // Narrow screens: info on top, full-width actions below. `lg`+: one row.
                                article class="flex flex-col gap-3 p-4 lg:flex-row lg:items-center lg:gap-4" {
                                    div class="flex min-w-0 flex-1 items-center gap-3" {
                                        (avatar(&s.estudiante.apellidos, true))
                                        div class="min-w-0 flex-1" {
                                            p class="line-clamp-2 text-[16px] leading-tight font-semibold text-gray-900 lg:truncate" { (s.estudiante.nombre_completo()) }
                                            p class="mt-0.5 truncate text-[13px] text-ios-gray" { (s.estudiante.grado()) }
                                            p class="truncate text-[12px] text-gray-400" {
                                                @match s.ultimo_pago {
                                                    Some(fecha) => { "Último pago " (fechas::corta(fecha)) }
                                                    None => { "Sin pagos" }
                                                }
                                            }
                                        }
                                        span class="shrink-0 self-start text-[17px] font-bold text-gray-900 tabular-nums lg:self-center" { (soles(s.saldo)) }
                                    }
                                    div class="grid grid-cols-2 gap-2 lg:flex lg:shrink-0" {
                                        a href=(format!("{}&desde=cobranza", rutas::comprobante(s.estudiante.id, fechas::lunes_de(hoy), s.estudiante.id_grado)))
                                            class="inline-flex min-h-11 items-center justify-center gap-1.5 rounded-xl border border-gray-200 bg-white px-3 text-[14px] font-medium text-gray-700 transition-colors hover:bg-gray-50 active:scale-95 lg:min-h-9"
                                            title="Ver comprobante" {
                                            (icono(Icono::Recibo, "h-4 w-4 text-gray-400")) "Comprobante"
                                        }
                                        button type="button" data-share-text=(mensaje(s))
                                            class="inline-flex min-h-11 items-center justify-center gap-1.5 rounded-xl border border-gray-200 bg-white px-3 text-[14px] font-semibold text-ios-blue transition-colors hover:bg-blue-50 active:scale-95 lg:min-h-9"
                                            title="Compartir con la familia" {
                                            (icono(Icono::Compartir, "h-4 w-4")) "Compartir"
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
