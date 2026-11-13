//! Date helpers. Dates are stored as `YYYY-MM-DD` text in SQLite.

use chrono::{Datelike, Local, NaiveDate, TimeDelta};

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

pub fn iso(fecha: NaiveDate) -> String {
    fecha.format("%Y-%m-%d").to_string()
}

pub fn corta(fecha: NaiveDate) -> String {
    fecha.format("%d/%m").to_string()
}

pub fn larga(fecha: NaiveDate) -> String {
    format!(
        "{} {}",
        DIAS_LARGOS[indice_dia(fecha)],
        fecha.format("%d/%m/%Y")
    )
}

pub fn etiqueta_semana(lunes: NaiveDate) -> String {
    format!(
        "Semana del {} al {}",
        corta(lunes),
        sumar_dias(lunes, 6).format("%d/%m/%Y")
    )
}
