use maud::{Markup, html};

pub fn page_header(title: &str, subtitle: Option<&str>) -> Markup {
    html! {
        header class="mb-6" {
            h1 class="text-2xl font-semibold tracking-tight" { (title) }
            @if let Some(subtitle) = subtitle {
                p class="text-sm text-slate-500" { (subtitle) }
            }
        }
    }
}

/// Money in soles, rounded to cents (never prints `-0.00`).
pub fn soles(valor: f64) -> String {
    let redondeado = (valor * 100.0).round() / 100.0;
    let redondeado = if redondeado == 0.0 { 0.0 } else { redondeado };
    format!("S/ {redondeado:.2}")
}

/// Money cell that shows a dash for zero.
pub fn soles_o_guion(valor: f64) -> Markup {
    html! {
        @if (valor * 100.0).round() == 0.0 {
            span class="text-slate-300" { "—" }
        } @else {
            (soles(valor))
        }
    }
}

pub fn alerta_ok(mensaje: &str) -> Markup {
    html! {
        div class="mb-4 rounded-md bg-emerald-50 px-4 py-2 text-sm text-emerald-800 ring-1 ring-emerald-200 print:hidden" {
            (mensaje)
        }
    }
}

pub fn badge_estado(activo: bool) -> Markup {
    html! {
        @if activo {
            span class="rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-800" { "Activo" }
        } @else {
            span class="rounded-full bg-slate-200 px-2 py-0.5 text-xs font-medium text-slate-600" { "Inactivo" }
        }
    }
}

/// Form that flips the `esta_activo` flag through `POST {action}`.
pub fn boton_estado(action: &str, activo: bool) -> Markup {
    html! {
        form method="post" action=(action) class="inline" {
            input type="hidden" name="activo" value=(if activo { "0" } else { "1" });
            button type="submit" class="btn-link" {
                (if activo { "Deshabilitar" } else { "Habilitar" })
            }
        }
    }
}

pub fn stat(etiqueta: &str, valor: &str, destacado: bool) -> Markup {
    html! {
        div class="card" {
            p class="text-xs font-medium tracking-wide text-slate-500 uppercase" { (etiqueta) }
            p class=(if destacado { "mt-1 text-xl font-semibold text-red-700 tabular-nums" } else { "mt-1 text-xl font-semibold tabular-nums" }) {
                (valor)
            }
        }
    }
}
