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

/// Positive amount rounded to two decimals. Accepts `1,50` and `1.50`.
pub fn monto(valor: &str, campo: &str) -> Result<f64, AppError> {
    let numero: f64 = valor
        .trim()
        .replace(',', ".")
        .parse()
        .map_err(|_| AppError::BadRequest(format!("«{campo}» debe ser un número.")))?;
    // Validate the stored (rounded) value: 0.001 would otherwise be saved as 0.00.
    let redondeado = (numero * 100.0).round() / 100.0;
    if !redondeado.is_finite() || redondeado <= 0.0 || redondeado > MONTO_MAXIMO {
        return Err(AppError::BadRequest(format!(
            "«{campo}» debe ser mayor que 0 y menor que {MONTO_MAXIMO}."
        )));
    }
    Ok(redondeado)
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    /// User-facing message of a rejected value.
    fn mensaje<T: std::fmt::Debug>(resultado: Result<T, AppError>) -> String {
        match resultado {
            Err(AppError::BadRequest(m)) => m,
            otro => panic!("expected a validation error, got {otro:?}"),
        }
    }

    // ── texto ───────────────────────────────────────────────────────────────

    #[rstest]
    #[case::simple("Ana", "Ana")]
    #[case::espacios_alrededor("  Ana María  ", "Ana María")]
    #[case::tildes_y_enie("José Ñuñez", "José Ñuñez")]
    #[case::apostrofe("D'Angelo", "D'Angelo")]
    #[case::guion("Ruiz-Díaz", "Ruiz-Díaz")]
    #[case::html_se_guarda_tal_cual("<b>Ana</b>", "<b>Ana</b>")]
    #[case::emoji("Ana 🍰", "Ana 🍰")]
    #[case::justo_en_el_maximo("ñandú", "ñandú")]
    fn texto_valido_se_acepta_sin_espacios_alrededor(
        #[case] entrada: &str,
        #[case] esperado: &str,
    ) {
        assert_eq!(texto(entrada, "Nombres", 10).unwrap(), esperado);
    }

    #[rstest]
    #[case::vacio("")]
    #[case::solo_espacios("   ")]
    #[case::tabulador_y_salto("\t\n")]
    fn texto_vacio_es_obligatorio(#[case] entrada: &str) {
        assert_eq!(
            mensaje(texto(entrada, "Nombres", 10)),
            "El campo «Nombres» es obligatorio."
        );
    }

    #[rstest]
    #[case::ascii("abcdefghijk")]
    #[case::caracteres_con_tilde("ñandúñandúñ")]
    fn texto_que_supera_el_maximo_se_rechaza(#[case] entrada: &str) {
        assert_eq!(
            mensaje(texto(entrada, "Nombres", 10)),
            "El campo «Nombres» admite como máximo 10 caracteres."
        );
    }

    #[test]
    fn el_maximo_cuenta_caracteres_y_no_bytes() {
        // 10 characters, 15 bytes in UTF-8.
        assert!(texto("ñandúñandú", "Nombres", 10).is_ok());
    }

    // ── monto ───────────────────────────────────────────────────────────────

    #[rstest]
    #[case::entero("5", 5.0)]
    #[case::punto_decimal("1.50", 1.5)]
    #[case::coma_decimal("1,50", 1.5)]
    #[case::espacios_alrededor(" 3.50 ", 3.5)]
    #[case::un_centimo("0.01", 0.01)]
    #[case::maximo("100000", 100_000.0)]
    #[case::redondeo_a_dos_decimales("12.345", 12.35)]
    fn monto_valido_se_acepta(#[case] entrada: &str, #[case] esperado: f64) {
        assert_eq!(monto(entrada, "Monto").unwrap(), esperado);
    }

    #[rstest]
    #[case::vacio("")]
    #[case::letras("cinco")]
    #[case::con_simbolo_de_moneda("S/ 5")]
    #[case::separador_de_miles("1,234.50")]
    #[case::dos_comas("1,2,3")]
    #[case::inyeccion_sql("5; DROP TABLE pagos")]
    fn monto_que_no_es_numero_se_rechaza(#[case] entrada: &str) {
        assert_eq!(
            mensaje(monto(entrada, "Monto")),
            "«Monto» debe ser un número."
        );
    }

    #[rstest]
    #[case::cero("0")]
    #[case::negativo("-5")]
    #[case::supera_el_maximo("100000.01")]
    #[case::infinito("inf")]
    #[case::no_es_un_numero("NaN")]
    #[case::redondea_a_cero("0.001")]
    #[case::redondea_a_cero_negativo("-0.001")]
    fn monto_fuera_de_rango_se_rechaza(#[case] entrada: &str) {
        assert_eq!(
            mensaje(monto(entrada, "Monto")),
            "«Monto» debe ser mayor que 0 y menor que 100000."
        );
    }

    // ── fecha ───────────────────────────────────────────────────────────────

    #[rstest]
    #[case::iso("2026-11-09", "2026-11-09")]
    #[case::espacios_alrededor(" 2026-11-09 ", "2026-11-09")]
    #[case::bisiesto("2028-02-29", "2028-02-29")]
    fn fecha_valida_se_acepta(#[case] entrada: &str, #[case] esperado: &str) {
        let esperado = NaiveDate::parse_from_str(esperado, "%Y-%m-%d").unwrap();
        assert_eq!(fecha(entrada, "Fecha").unwrap(), esperado);
    }

    #[rstest]
    #[case::vacia("")]
    #[case::bisiesto_inexistente("2026-02-29")]
    #[case::mes_13("2026-13-01")]
    #[case::formato_dia_mes_anio("09/11/2026")]
    #[case::dia_en_ingles("Monday")]
    #[case::con_hora("2026-11-09T10:00")]
    #[case::html("<script>")]
    fn fecha_invalida_se_rechaza(#[case] entrada: &str) {
        assert_eq!(
            mensaje(fecha(entrada, "Fecha")),
            "«Fecha» no es una fecha válida."
        );
    }

    // ── cantidad ────────────────────────────────────────────────────────────

    #[rstest]
    #[case::vacia_es_cero("", 0)]
    #[case::solo_espacios_es_cero("   ", 0)]
    #[case::cero("0", 0)]
    #[case::uno("1", 1)]
    #[case::espacios_alrededor(" 7 ", 7)]
    #[case::cero_a_la_izquierda("05", 5)]
    #[case::signo_mas("+3", 3)]
    #[case::maximo("999", 999)]
    fn cantidad_valida_se_acepta(#[case] entrada: &str, #[case] esperado: i64) {
        assert_eq!(cantidad(entrada).unwrap(), esperado);
    }

    #[rstest]
    #[case::negativa("-1")]
    #[case::supera_el_maximo("1000")]
    #[case::decimal("1.5")]
    #[case::decimal_con_coma("1,5")]
    #[case::notacion_cientifica("1e2")]
    #[case::letras("dos")]
    #[case::digitos_arabes("٣")]
    #[case::desborda_i64("99999999999999999999")]
    fn cantidad_invalida_se_rechaza(#[case] entrada: &str) {
        let esperado = format!(
            "La cantidad «{}» no es válida: usa un entero entre 0 y 999.",
            entrada.trim()
        );
        assert_eq!(mensaje(cantidad(entrada)), esperado);
    }
}
