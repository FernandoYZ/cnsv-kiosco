//! Date helpers. Dates are stored as `YYYY-MM-DD` text in SQLite.

use chrono::{Datelike, Local, NaiveDate, TimeDelta};

/// Monday-first short day names (index = `indice_dia`).
pub const DIAS_CORTOS: [&str; 7] = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];
const DIAS_LARGOS: [&str; 7] = [
    "lunes",
    "martes",
    "miércoles",
    "jueves",
    "viernes",
    "sábado",
    "domingo",
];
const MESES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];

/// Days always shown: Monday to Friday. Saturday and Sunday appear only with data.
const DIAS_HABILES: usize = 5;

/// Today in the server's local time zone (set `TZ=America/Lima` in systemd).
pub fn hoy() -> NaiveDate {
    Local::now().date_naive()
}

/// Parses `YYYY-MM-DD`; falls back to today when missing or invalid.
pub fn parse_o_hoy(valor: Option<&str>) -> NaiveDate {
    valor
        .and_then(|v| NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d").ok())
        .unwrap_or_else(hoy)
}

pub fn lunes_de(fecha: NaiveDate) -> NaiveDate {
    fecha - TimeDelta::days(i64::from(fecha.weekday().num_days_from_monday()))
}

pub fn sumar_dias(fecha: NaiveDate, dias: i64) -> NaiveDate {
    fecha + TimeDelta::days(dias)
}

/// Index 0 (Monday) to 6 (Sunday).
pub fn indice_dia(fecha: NaiveDate) -> usize {
    fecha.weekday().num_days_from_monday() as usize
}

/// Monday–Friday of the week plus any weekend day for which `tiene_datos` is true.
pub fn dias_visibles(lunes: NaiveDate, tiene_datos: impl Fn(usize) -> bool) -> Vec<NaiveDate> {
    (0..7)
        .filter(|&d| d < DIAS_HABILES || tiene_datos(d))
        .map(|d| sumar_dias(lunes, d as i64))
        .collect()
}

fn mes(fecha: NaiveDate) -> &'static str {
    MESES[fecha.month0() as usize]
}

pub fn iso(fecha: NaiveDate) -> String {
    fecha.format("%Y-%m-%d").to_string()
}

/// `Lun 09/11`
pub fn corta(fecha: NaiveDate) -> String {
    format!(
        "{} {}",
        DIAS_CORTOS[indice_dia(fecha)],
        fecha.format("%d/%m")
    )
}

/// `lunes, 9 de noviembre de 2026`
pub fn larga(fecha: NaiveDate) -> String {
    format!(
        "{}, {} de {} de {}",
        DIAS_LARGOS[indice_dia(fecha)],
        fecha.day(),
        mes(fecha),
        fecha.year()
    )
}

/// `Lunes - 09/11`, used by the day selector.
pub fn opcion_dia(fecha: NaiveDate) -> String {
    let dia = DIAS_LARGOS[indice_dia(fecha)];
    let mut letras = dia.chars();
    let capitalizado: String = letras
        .next()
        .map(|c| c.to_uppercase().chain(letras).collect())
        .unwrap_or_default();
    format!("{capitalizado} - {}", fecha.format("%d/%m"))
}

/// (`9 noviembre`, `13 noviembre, 2026`) for the weekly header.
pub fn rango_semana(lunes: NaiveDate) -> (String, String) {
    let viernes = sumar_dias(lunes, 4);
    (
        format!("{} {}", lunes.day(), mes(lunes)),
        format!("{} {}, {}", viernes.day(), mes(viernes), viernes.year()),
    )
}

/// `Semana del 9 al 13 de noviembre`
pub fn etiqueta_semana(lunes: NaiveDate) -> String {
    let viernes = sumar_dias(lunes, 4);
    format!(
        "Semana del {} al {} de {}",
        lunes.day(),
        viernes.day(),
        mes(viernes)
    )
}

/// `9-13/noviembre/2026`, used in the receipt.
pub fn periodo(lunes: NaiveDate) -> String {
    let viernes = sumar_dias(lunes, 4);
    format!(
        "{}-{}/{}/{}",
        lunes.day(),
        viernes.day(),
        mes(viernes),
        viernes.year()
    )
}
