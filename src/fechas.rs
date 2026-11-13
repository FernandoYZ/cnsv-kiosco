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

/// Days always shown: Monday to Saturday, as the Go app. Sunday appears only with data.
const DIAS_HABILES: usize = 6;

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
    format!(
        "{} - {}",
        capitalizar(DIAS_LARGOS[indice_dia(fecha)]),
        fecha.format("%d/%m")
    )
}

fn capitalizar(texto: &str) -> String {
    let mut letras = texto.chars();
    letras
        .next()
        .map(|c| c.to_uppercase().chain(letras).collect())
        .unwrap_or_default()
}

/// Last day of the kiosk week (Saturday).
fn sabado(lunes: NaiveDate) -> NaiveDate {
    sumar_dias(lunes, 5)
}

/// (`26 Octubre`, `31 Octubre, 2026`) for the weekly header, as the Go app.
pub fn rango_semana(lunes: NaiveDate) -> (String, String) {
    let fin = sabado(lunes);
    (
        format!("{} {}", lunes.day(), capitalizar(mes(lunes))),
        format!("{} {}, {}", fin.day(), capitalizar(mes(fin)), fin.year()),
    )
}

/// `Semana del 26 al 31 de Octubre`
pub fn etiqueta_semana(lunes: NaiveDate) -> String {
    let fin = sabado(lunes);
    format!(
        "Semana del {} al {} de {}",
        lunes.day(),
        fin.day(),
        capitalizar(mes(fin))
    )
}

/// `26-31/Octubre/2026`, used in the receipt.
pub fn periodo(lunes: NaiveDate) -> String {
    let fin = sabado(lunes);
    format!(
        "{}-{}/{}/{}",
        lunes.day(),
        fin.day(),
        capitalizar(mes(fin)),
        fin.year()
    )
}
