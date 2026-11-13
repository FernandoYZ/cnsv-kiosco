//! Validation of submitted form fields. Errors are user-facing messages.

use chrono::NaiveDate;

use crate::error::AppError;

const MONTO_MAXIMO: f64 = 100_000.0;
const CANTIDAD_MAXIMA: i64 = 999;

pub fn texto(valor: &str, campo: &str, max: usize) -> Result<String, AppError> {
    let valor = valor.trim();
    if valor.is_empty() {
        return Err(AppError::BadRequest(format!(
            "El campo «{campo}» es obligatorio."
        )));
    }
    if valor.chars().count() > max {
        return Err(AppError::BadRequest(format!(
            "El campo «{campo}» admite como máximo {max} caracteres."
        )));
    }
    Ok(valor.to_owned())
}

/// Positive amount with at most two decimals. Accepts `1,50` and `1.50`.
pub fn monto(valor: &str, campo: &str) -> Result<f64, AppError> {
    let numero: f64 = valor
        .trim()
        .replace(',', ".")
        .parse()
        .map_err(|_| AppError::BadRequest(format!("«{campo}» debe ser un número.")))?;
    if !numero.is_finite() || numero <= 0.0 || numero > MONTO_MAXIMO {
        return Err(AppError::BadRequest(format!(
            "«{campo}» debe ser mayor que 0 y menor que {MONTO_MAXIMO}."
        )));
    }
    Ok((numero * 100.0).round() / 100.0)
}

pub fn fecha(valor: &str, campo: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(valor.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest(format!("«{campo}» no es una fecha válida.")))
}

/// Grid quantity: empty means 0.
pub fn cantidad(valor: &str) -> Result<i64, AppError> {
    let valor = valor.trim();
    if valor.is_empty() {
        return Ok(0);
    }
    match valor.parse::<i64>() {
        Ok(n) if (0..=CANTIDAD_MAXIMA).contains(&n) => Ok(n),
        _ => Err(AppError::BadRequest(format!(
            "La cantidad «{valor}» no es válida: usa un entero entre 0 y {CANTIDAD_MAXIMA}."
        ))),
    }
}
