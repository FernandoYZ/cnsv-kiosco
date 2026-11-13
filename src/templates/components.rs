//! Reusable HTML pieces of the iOS-inspired design.

use chrono::NaiveDate;
use maud::{Markup, PreEscaped, html};

use crate::fechas;

#[derive(Clone, Copy)]
pub enum Icono {
    Estudiantes,
    Productos,
    Registro,
    Recibo,
    Inicio,
    ChevronIzquierda,
    ChevronDerecha,
    Buscar,
    Persona,
    Check,
    Dinero,
    Portapapeles,
    Alerta,
    Compartir,
    Salir,
    Calculadora,
}

fn trazos(icono: Icono) -> &'static str {
    match icono {
        Icono::Estudiantes => {
            r#"<path d="M15.5 7.5C15.5 9.433 13.933 11 12 11C10.067 11 8.5 9.433 8.5 7.5C8.5 5.567 10.067 4 12 4C13.933 4 15.5 5.567 15.5 7.5Z" fill="currentColor"/><path opacity="0.4" d="M19.5 7.5C19.5 8.88071 18.3807 10 17 10C15.6193 10 14.5 8.88071 14.5 7.5C14.5 6.11929 15.6193 5 17 5C18.3807 5 19.5 6.11929 19.5 7.5Z" fill="currentColor"/><path opacity="0.4" d="M4.5 7.5C4.5 8.88071 5.61929 10 7 10C8.38071 10 9.5 8.88071 9.5 7.5C9.5 6.11929 8.38071 5 7 5C5.61929 5 4.5 6.11929 4.5 7.5Z" fill="currentColor"/><path d="M18 16.5C18 18.433 15.3137 20 12 20C8.68629 20 6 18.433 6 16.5C6 14.567 8.68629 13 12 13C15.3137 13 18 14.567 18 16.5Z" fill="currentColor"/><path opacity="0.4" d="M22 16.5C22 17.8807 20.2091 19 18 19C15.7909 19 14 17.8807 14 16.5C14 15.1193 15.7909 14 18 14C20.2091 14 22 15.1193 22 16.5Z" fill="currentColor"/><path opacity="0.4" d="M2 16.5C2 17.8807 3.79086 19 6 19C8.20914 19 10 17.8807 10 16.5C10 15.1193 8.20914 14 6 14C3.79086 14 2 15.1193 2 16.5Z" fill="currentColor"/>"#
        }
        Icono::Productos => {
            r#"<path d="M8.42229 20.6181C10.1779 21.5395 11.0557 22.0001 12 22.0001V12.0001L2.63802 7.07275C2.62423 7.09491 2.6107 7.11727 2.5974 7.13986C2 8.15436 2 9.41678 2 11.9416V12.0586C2 14.5834 2 15.8459 2.5974 16.8604C3.19479 17.8749 4.27063 18.4395 6.42229 19.5686L8.42229 20.6181Z" fill="currentColor"/><path opacity="0.7" d="M17.5774 4.43152L15.5774 3.38197C13.8218 2.46066 12.944 2 11.9997 2C11.0554 2 10.1776 2.46066 8.42197 3.38197L6.42197 4.43152C4.31821 5.53552 3.24291 6.09982 2.6377 7.07264L11.9997 12L21.3617 7.07264C20.7564 6.09982 19.6811 5.53552 17.5774 4.43152Z" fill="currentColor"/><path opacity="0.5" d="M21.4026 7.13986C21.3893 7.11727 21.3758 7.09491 21.362 7.07275L12 12.0001V22.0001C12.9443 22.0001 13.8221 21.5395 15.5777 20.6181L17.5777 19.5686C19.7294 18.4395 20.8052 17.8749 21.4026 16.8604C22 15.8459 22 14.5834 22 12.0586V11.9416C22 9.41678 22 8.15436 21.4026 7.13986Z" fill="currentColor"/>"#
        }
        Icono::Registro => {
            r#"<path opacity="0.5" d="M3 10C3 6.22876 3 4.34315 4.17157 3.17157C5.34315 2 7.22876 2 11 2H13C16.7712 2 18.6569 2 19.8284 3.17157C21 4.34315 21 6.22876 21 10V14C21 17.7712 21 19.6569 19.8284 20.8284C18.6569 22 16.7712 22 13 22H11C7.22876 22 5.34315 22 4.17157 20.8284C3 19.6569 3 17.7712 3 14V10Z" fill="currentColor"/><path fill-rule="evenodd" clip-rule="evenodd" d="M7.25 9C7.25 8.58579 7.58579 8.25 8 8.25H14.5C14.9142 8.25 15.25 8.58579 15.25 9C15.25 9.41421 14.9142 9.75 14.5 9.75H8C7.58579 9.75 7.25 9.41421 7.25 9ZM7.25 13C7.25 12.5858 7.58579 12.25 8 12.25H11C11.4142 12.25 11.75 12.5858 11.75 13C11.75 13.4142 11.4142 13.75 11 13.75H8C7.58579 13.75 7.25 13.4142 7.25 13ZM7.25 17C7.25 16.5858 7.58579 16.25 8 16.25H9.5C9.91421 16.25 10.25 16.5858 10.25 17C10.25 17.4142 9.91421 17.75 9.5 17.75H8C7.58579 17.75 7.25 17.4142 7.25 17Z" fill="currentColor"/>"#
        }
        Icono::Recibo => {
            r#"<path d="M5 3.5h9.5L19 8v12.5H5z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M14 3.5V8h5" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M8 13h8M8 16.5h5.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>"#
        }
        Icono::Inicio => {
            r#"<path d="M13.1061 22H10.8939C7.44737 22 5.72409 22 4.54903 20.9882C3.37396 19.9764 3.13025 18.2827 2.64284 14.8952L2.36407 12.9579C1.98463 10.3208 1.79491 9.00229 2.33537 7.87495C2.87583 6.7476 4.02619 6.06234 6.32691 4.69181L7.71175 3.86687C9.80104 2.62229 10.8457 2 12 2C13.1543 2 14.199 2.62229 16.2882 3.86687L17.6731 4.69181C19.9738 6.06234 21.1242 6.7476 21.6646 7.87495C22.2051 9.00229 22.0154 10.3208 21.6359 12.9579L21.3572 14.8952C20.8697 18.2827 20.626 19.9764 19.451 20.9882C18.2759 22 16.5526 22 13.1061 22ZM8.39757 15.5532C8.64423 15.2204 9.11395 15.1506 9.44671 15.3973C10.1751 15.9371 11.0542 16.2498 12.0001 16.2498C12.946 16.2498 13.8251 15.9371 14.5535 15.3973C14.8863 15.1506 15.356 15.2204 15.6026 15.5532C15.8493 15.8859 15.7795 16.3557 15.4467 16.6023C14.4743 17.3231 13.2851 17.7498 12.0001 17.7498C10.7151 17.7498 9.5259 17.3231 8.55349 16.6023C8.22072 16.3557 8.15092 15.8859 8.39757 15.5532Z" fill-rule="evenodd" clip-rule="evenodd" fill="currentColor"/>"#
        }
        Icono::ChevronIzquierda => {
            r#"<path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M15 19l-7-7 7-7"/>"#
        }
        Icono::ChevronDerecha => {
            r#"<path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7"/>"#
        }
        Icono::Buscar => {
            r#"<path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>"#
        }
        Icono::Persona => {
            r#"<path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"/>"#
        }
        Icono::Check => {
            r#"<path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7"/>"#
        }
        Icono::Dinero => {
            r#"<path d="M12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22ZM12.75 6C12.75 5.58579 12.4142 5.25 12 5.25C11.5858 5.25 11.25 5.58579 11.25 6V6.31673C9.61957 6.60867 8.25 7.83361 8.25 9.5C8.25 11.4172 10.0628 12.75 12 12.75C13.3765 12.75 14.25 13.6557 14.25 14.5C14.25 15.3443 13.3765 16.25 12 16.25C10.6235 16.25 9.75 15.3443 9.75 14.5C9.75 14.0858 9.41421 13.75 9 13.75C8.58579 13.75 8.25 14.0858 8.25 14.5C8.25 16.1664 9.61957 17.3913 11.25 17.6833V18C11.25 18.4142 11.5858 18.75 12 18.75C12.4142 18.75 12.75 18.4142 12.75 18V17.6833C14.3804 17.3913 15.75 16.1664 15.75 14.5C15.75 12.5828 13.9372 11.25 12 11.25C10.6235 11.25 9.75 10.3443 9.75 9.5C9.75 8.65573 10.6235 7.75 12 7.75C13.3765 7.75 14.25 8.65573 14.25 9.5C14.25 9.91421 14.5858 10.25 15 10.25C15.4142 10.25 15.75 9.91421 15.75 9.5C15.75 7.83361 14.3804 6.60867 12.75 6.31673V6Z" fill-rule="evenodd" clip-rule="evenodd" fill="currentColor"/>"#
        }
        Icono::Portapapeles => {
            r#"<path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>"#
        }
        Icono::Alerta => {
            r#"<path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.2" d="M12 9v2m0 4v2m0-12a9 9 0 110 18 9 9 0 010-18z"/>"#
        }
        Icono::Compartir => {
            r#"<path opacity="0.5" stroke="currentColor" stroke-linecap="round" stroke-width="2" d="M4 13v4a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-4"/><path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15V4m0 0L8 8m4-4 4 4"/>"#
        }
        Icono::Salir => {
            r#"<path d="M12.75 2C12.75 1.58579 12.4142 1.25 12 1.25C11.5858 1.25 11.25 1.58579 11.25 2V6C11.25 6.41421 11.5858 6.75 12 6.75C12.4142 6.75 12.75 6.41421 12.75 6V2Z" fill="currentColor"/><path d="M8.7919 4.39692C9.17345 4.23571 9.35208 3.79571 9.19087 3.41416C9.02966 3.0326 8.58966 2.85398 8.2081 3.01519C4.70832 4.49386 2.25 7.95905 2.25 12.0001C2.25 17.3849 6.61522 21.7501 12 21.7501C17.3848 21.7501 21.75 17.3849 21.75 12.0001C21.75 7.95905 19.2917 4.49386 15.7919 3.01519C15.4103 2.85398 14.9703 3.0326 14.8091 3.41416C14.6479 3.79571 14.8265 4.23571 15.2081 4.39692C18.1722 5.64927 20.25 8.58293 20.25 12.0001C20.25 16.5565 16.5563 20.2501 12 20.2501C7.44365 20.2501 3.75 16.5565 3.75 12.0001C3.75 8.58293 5.82779 5.64927 8.7919 4.39692Z" fill="currentColor"/>"#
        }
        Icono::Calculadora => {
            r#"<path d="M12 22C7.28595 22 4.92893 22 3.46447 20.5355C2 19.0711 2 16.714 2 12C2 7.28595 2 4.92893 3.46447 3.46447C4.92893 2 7.28595 2 12 2C16.714 2 19.0711 2 20.5355 3.46447C22 4.92893 22 7.28595 22 12C22 16.714 22 19.0711 20.5355 20.5355C19.0711 22 16.714 22 12 22ZM8.75 6.49998C8.75 6.08576 8.41421 5.74998 8 5.74998C7.58579 5.74998 7.25 6.08576 7.25 6.49998L7.25 7.74999H6C5.58579 7.74999 5.25 8.08578 5.25 8.49999C5.25 8.91421 5.58579 9.24999 6 9.24999L7.25 9.24999V10.5C7.25 10.9142 7.58579 11.25 8 11.25C8.41421 11.25 8.75 10.9142 8.75 10.5V9.24999H10C10.4142 9.24999 10.75 8.91421 10.75 8.49999C10.75 8.08578 10.4142 7.74999 10 7.74999H8.75L8.75 6.49998ZM14 7.74998C13.5858 7.74998 13.25 8.08576 13.25 8.49998C13.25 8.91419 13.5858 9.24998 14 9.24998H18C18.4142 9.24998 18.75 8.91419 18.75 8.49998C18.75 8.08576 18.4142 7.74998 18 7.74998H14ZM14 13.75C13.5858 13.75 13.25 14.0858 13.25 14.5C13.25 14.9142 13.5858 15.25 14 15.25H18C18.4142 15.25 18.75 14.9142 18.75 14.5C18.75 14.0858 18.4142 13.75 18 13.75H14ZM7.03033 13.9697C6.73744 13.6768 6.26256 13.6768 5.96967 13.9697C5.67678 14.2626 5.67678 14.7374 5.96967 15.0303L6.93935 16L5.96968 16.9697C5.67679 17.2626 5.67679 17.7374 5.96968 18.0303C6.26258 18.3232 6.73745 18.3232 7.03034 18.0303L8.00001 17.0607L8.96966 18.0303C9.26255 18.3232 9.73742 18.3232 10.0303 18.0303C10.3232 17.7374 10.3232 17.2626 10.0303 16.9697L9.06067 16L10.0303 15.0303C10.3232 14.7374 10.3232 14.2626 10.0303 13.9697C9.73744 13.6768 9.26256 13.6768 8.96967 13.9697L8.00001 14.9393L7.03033 13.9697ZM14 16.75C13.5858 16.75 13.25 17.0858 13.25 17.5C13.25 17.9142 13.5858 18.25 14 18.25H18C18.4142 18.25 18.75 17.9142 18.75 17.5C18.75 17.0858 18.4142 16.75 18 16.75H14Z" fill-rule="evenodd" clip-rule="evenodd" fill="currentColor"/>"#
        }
    }
}

pub fn icono(icono: Icono, clase: &str) -> Markup {
    html! {
        svg class=(clase) fill="none" viewBox="0 0 24 24" aria-hidden="true" { (PreEscaped(trazos(icono))) }
    }
}

/// `12.50` (no currency symbol), rounded to cents and never `-0.00`.
pub fn monto(valor: f64) -> String {
    let redondeado = (valor * 100.0).round() / 100.0;
    let redondeado = if redondeado == 0.0 { 0.0 } else { redondeado };
    format!("{redondeado:.2}")
}

/// `S/ 12.50`
pub fn soles(valor: f64) -> String {
    format!("S/ {}", monto(valor))
}

pub fn es_cero(valor: f64) -> bool {
    (valor * 100.0).round() == 0.0
}

/// Sticky top bar: back link, centered title and an optional right slot.
pub fn ios_nav(atras: (&str, &str), titulo: &str, derecha: Markup) -> Markup {
    html! {
        nav class="ios-nav" {
            div class="mx-auto flex max-w-2xl items-center justify-between gap-3 lg:max-w-6xl" {
                a href=(atras.0) class="ios-link shrink-0" {
                    (icono(Icono::ChevronIzquierda, "-ml-2 h-6 w-6"))
                    span class="text-[17px] font-medium" { (atras.1) }
                }
                h2 class="truncate text-[17px] font-semibold" { (titulo) }
                div class="flex min-w-12 shrink-0 justify-end" { (derecha) }
            }
        }
    }
}

/// Large page title with grey subtitle.
pub fn encabezado(titulo: &str, subtitulo: &str) -> Markup {
    html! {
        header class="mb-6 sm:mb-8" {
            h1 class="text-[28px] leading-tight font-bold tracking-tight text-gray-900 sm:text-[34px]" { (titulo) }
            p class="mt-1 text-[15px] font-medium text-ios-gray sm:text-[17px]" { (subtitulo) }
        }
    }
}

pub fn estado_vacio(icono_vacio: Icono, titulo: &str, texto: &str) -> Markup {
    html! {
        div class="px-6 py-16 text-center sm:py-20" {
            div class="mx-auto mb-4 flex h-20 w-20 items-center justify-center rounded-full bg-ios-bg" {
                (icono(icono_vacio, "h-10 w-10 text-ios-gray-light"))
            }
            h3 class="text-[19px] font-bold text-gray-900" { (titulo) }
            p class="mx-auto mt-2 max-w-[260px] text-[15px] text-ios-gray" { (texto) }
        }
    }
}

/// First letter of the surname inside a circle.
pub fn avatar(texto: &str, activo: bool) -> Markup {
    let inicial: String = texto
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default();
    html! {
        div class=(if activo { "avatar" } else { "avatar bg-gray-100 text-gray-400 opacity-50" }) { (inicial) }
    }
}

/// Mobile floating bottom navigation (hidden from `sm` up).
pub fn bottom_nav() -> Markup {
    let item = |href: &str, ico: Icono, etiqueta: &str| {
        html! {
            a href=(href) class="flex h-full flex-1 flex-col items-center justify-center gap-1.5 text-ios-gray transition-transform active:scale-90 active:text-ios-blue" {
                (icono(ico, "h-7 w-7"))
                span class="text-[10px] font-bold tracking-wider uppercase" { (etiqueta) }
            }
        }
    };
    html! {
        nav id="bottom-nav" class="fixed right-6 bottom-6 left-6 z-50 rounded-[28px] border border-white/20 bg-white/80 shadow-[0_20px_50px_rgba(0,0,0,0.15)] backdrop-blur-2xl transition-transform duration-300 sm:hidden print:hidden"
            style="padding-bottom: env(safe-area-inset-bottom)" {
            div class="flex h-20 items-center justify-around px-2" {
                // Shown only on the weekly view, so "Inicio" would point to itself.
                (item("/estudiantes", Icono::Estudiantes, "Alumnos"))
                (item("/productos", Icono::Productos, "Productos"))
                (item("/registro", Icono::Registro, "Registro"))
                (item("/cobranza", Icono::Dinero, "Cobranza"))
                (item("/cierre", Icono::Calculadora, "Cierre"))
            }
        }
    }
}

/// Day `<select>` for the week of `fecha` (Monday–Saturday) that navigates on change.
pub fn selector_dia(fecha: NaiveDate, url: impl Fn(NaiveDate) -> String) -> Markup {
    let lunes = fechas::lunes_de(fecha);
    let mut dias: Vec<NaiveDate> = (0..6).map(|d| fechas::sumar_dias(lunes, d)).collect();
    if !dias.contains(&fecha) {
        dias.push(fecha);
    }
    html! {
        div class="px-3 py-3 sm:px-4 sm:py-4 print:hidden" {
            div class="mx-auto flex max-w-2xl items-center gap-2 lg:max-w-6xl" {
                a href=(url(fechas::sumar_dias(lunes, -7))) class="rounded-xl p-2.5 text-gray-500 hover:bg-white hover:text-ios-blue" title="Semana anterior" {
                    (icono(Icono::ChevronIzquierda, "h-5 w-5"))
                }
                select data-navigate aria-label="Día" class="select-ios" {
                    @for dia in &dias {
                        option value=(url(*dia)) selected[*dia == fecha] { (fechas::opcion_dia(*dia)) }
                    }
                }
                a href=(url(fechas::sumar_dias(lunes, 7))) class="rounded-xl p-2.5 text-gray-500 hover:bg-white hover:text-ios-blue" title="Semana siguiente" {
                    (icono(Icono::ChevronDerecha, "h-5 w-5"))
                }
            }
        }
    }
}

/// Flash messages passed as `?ok=<code>` after a redirect.
pub fn mensaje_flash(codigo: Option<&str>) -> Option<&'static str> {
    Some(match codigo? {
        "consumo" => "Consumo guardado",
        "pago" => "Abono registrado",
        "pago-duplicado" => "Ese abono ya estaba registrado: no se guardó de nuevo",
        "pago-eliminado" => "Pago eliminado",
        "estudiante-creado" => "Estudiante registrado",
        "estudiante-actualizado" => "Estudiante actualizado",
        "estudiante-estado" => "Estado del estudiante actualizado",
        "producto-creado" => "Producto agregado",
        "producto-actualizado" => "Producto actualizado",
        "producto-estado" => "Estado del producto actualizado",
        _ => return None,
    })
}
