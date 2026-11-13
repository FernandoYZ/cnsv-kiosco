use axum::extract::{Query, State};
use maud::{Markup, html};
use serde::Deserialize;

use super::repository::{self, Saldo};
use crate::{
    error::AppError,
    estudiantes::{GRADOS, grado_desde},
    fechas,
    pagos::handler::url_comprobante,
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
        s.nombres,
        s.apellidos,
        s.grado(),
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
            main class="mx-auto max-w-2xl px-4 pt-6 pb-10 lg:max-w-4xl" {
                header class="mb-6" {
                    h1 class="text-[34px] leading-tight font-bold tracking-tight text-gray-900" { "Cobranza" }
                    p class="mt-1 text-[17px] font-medium text-ios-gray" {
                        (deudores.len()) " con deuda · " (al_dia) " al día"
                    }
                }
                div class="ios-card mb-6 flex items-center justify-between p-5" {
                    span class="text-[15px] font-medium text-gray-500" { "Total por cobrar" }
                    span class="text-[28px] font-black text-ios-red tabular-nums" { (soles(total)) }
                }
                nav class="no-scrollbar mb-6 flex gap-2 overflow-x-auto px-1 pb-1" aria-label="Filtrar por grado" {
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
                                div class="flex items-center justify-between gap-3 p-4" {
                                    div class="flex min-w-0 items-center gap-3" {
                                        (avatar(&s.apellidos, true))
                                        div class="min-w-0" {
                                            p class="truncate text-[16px] leading-tight font-semibold text-gray-900" { (s.nombre_completo()) }
                                            p class="text-[13px] font-medium text-ios-gray" {
                                                (s.grado()) " · "
                                                @match s.ultimo_pago {
                                                    Some(fecha) => { "último pago " (fechas::corta(fecha)) }
                                                    None => { "sin pagos" }
                                                }
                                            }
                                        }
                                    }
                                    div class="flex shrink-0 flex-col items-end gap-1" {
                                        span class="text-[17px] font-bold text-ios-red tabular-nums" { (soles(s.saldo)) }
                                        div class="flex items-center gap-1" {
                                            a href=(format!("{}&desde=cobranza", url_comprobante(s.id_estudiante, fechas::lunes_de(hoy), s.id_grado.unwrap_or(0))))
                                                class="btn-text px-2 text-[13px] text-ios-blue hover:bg-blue-50" title="Ver comprobante" {
                                                (icono(Icono::Recibo, "inline h-4 w-4")) " Comprobante"
                                            }
                                            button type="button" data-share-text=(mensaje(s))
                                                class="btn-text px-2 text-[13px] text-ios-green hover:bg-green-50" title="Compartir con la familia" {
                                                "Compartir"
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
