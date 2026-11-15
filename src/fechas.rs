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

/// Monday–Saturday of the week, plus Sunday only when `tiene_datos(6)` is true.
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

/// `Semana del 26 al 31 de Octubre`, or
/// `Semana del 30 de Noviembre al 5 de Diciembre` when the week crosses a month.
pub fn etiqueta_semana(lunes: NaiveDate) -> String {
    let fin = sabado(lunes);
    if lunes.month() == fin.month() {
        format!(
            "Semana del {} al {} de {}",
            lunes.day(),
            fin.day(),
            capitalizar(mes(fin))
        )
    } else {
        format!(
            "Semana del {} de {} al {} de {}",
            lunes.day(),
            capitalizar(mes(lunes)),
            fin.day(),
            capitalizar(mes(fin))
        )
    }
}

/// `26-31/Octubre/2026` in the receipt; a week crossing a month shows both
/// (`30/Noviembre-5/Diciembre/2026`), and crossing a year shows both years.
pub fn periodo(lunes: NaiveDate) -> String {
    let fin = sabado(lunes);
    let hasta = format!("{}/{}/{}", fin.day(), capitalizar(mes(fin)), fin.year());
    if lunes.year() != fin.year() {
        format!(
            "{}/{}/{}-{hasta}",
            lunes.day(),
            capitalizar(mes(lunes)),
            lunes.year()
        )
    } else if lunes.month() != fin.month() {
        format!("{}/{}-{hasta}", lunes.day(), capitalizar(mes(lunes)))
    } else {
        format!("{}-{hasta}", lunes.day())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn fecha(texto: &str) -> NaiveDate {
        NaiveDate::parse_from_str(texto, "%Y-%m-%d").unwrap()
    }

    /// Monday of the reference week: 2026-11-09 (Monday) to 2026-11-15 (Sunday).
    const LUNES: &str = "2026-11-09";

    // ── Each day of the week, one by one ────────────────────────────────────

    #[rstest]
    #[case::lunes("2026-11-09")]
    #[case::martes("2026-11-10")]
    #[case::miercoles("2026-11-11")]
    #[case::jueves("2026-11-12")]
    #[case::viernes("2026-11-13")]
    #[case::sabado("2026-11-14")]
    #[case::domingo("2026-11-15")]
    fn cada_dia_pertenece_a_la_semana_de_su_lunes(#[case] dia: &str) {
        assert_eq!(lunes_de(fecha(dia)), fecha(LUNES));
    }

    #[rstest]
    #[case::lunes("2026-11-09", 0)]
    #[case::martes("2026-11-10", 1)]
    #[case::miercoles("2026-11-11", 2)]
    #[case::jueves("2026-11-12", 3)]
    #[case::viernes("2026-11-13", 4)]
    #[case::sabado("2026-11-14", 5)]
    #[case::domingo("2026-11-15", 6)]
    fn el_indice_va_de_lunes_0_a_domingo_6(#[case] dia: &str, #[case] indice: usize) {
        assert_eq!(indice_dia(fecha(dia)), indice);
    }

    #[rstest]
    #[case::lunes("2026-11-09", "Lun 09/11")]
    #[case::martes("2026-11-10", "Mar 10/11")]
    #[case::miercoles("2026-11-11", "Mié 11/11")]
    #[case::jueves("2026-11-12", "Jue 12/11")]
    #[case::viernes("2026-11-13", "Vie 13/11")]
    #[case::sabado("2026-11-14", "Sáb 14/11")]
    #[case::domingo("2026-11-15", "Dom 15/11")]
    fn formato_corto_de_cada_dia(#[case] dia: &str, #[case] esperado: &str) {
        assert_eq!(corta(fecha(dia)), esperado);
    }

    #[rstest]
    #[case::lunes("2026-11-09", "lunes, 9 de noviembre de 2026")]
    #[case::martes("2026-11-10", "martes, 10 de noviembre de 2026")]
    #[case::miercoles("2026-11-11", "miércoles, 11 de noviembre de 2026")]
    #[case::jueves("2026-11-12", "jueves, 12 de noviembre de 2026")]
    #[case::viernes("2026-11-13", "viernes, 13 de noviembre de 2026")]
    #[case::sabado("2026-11-14", "sábado, 14 de noviembre de 2026")]
    #[case::domingo("2026-11-15", "domingo, 15 de noviembre de 2026")]
    fn formato_largo_de_cada_dia(#[case] dia: &str, #[case] esperado: &str) {
        assert_eq!(larga(fecha(dia)), esperado);
    }

    #[rstest]
    #[case::lunes("2026-11-09", "Lunes - 09/11")]
    #[case::martes("2026-11-10", "Martes - 10/11")]
    #[case::miercoles("2026-11-11", "Miércoles - 11/11")]
    #[case::jueves("2026-11-12", "Jueves - 12/11")]
    #[case::viernes("2026-11-13", "Viernes - 13/11")]
    #[case::sabado("2026-11-14", "Sábado - 14/11")]
    #[case::domingo("2026-11-15", "Domingo - 15/11")]
    fn opcion_del_selector_de_cada_dia(#[case] dia: &str, #[case] esperado: &str) {
        assert_eq!(opcion_dia(fecha(dia)), esperado);
    }

    #[rstest]
    #[case::lunes("2026-11-09", true)]
    #[case::martes("2026-11-10", true)]
    #[case::miercoles("2026-11-11", true)]
    #[case::jueves("2026-11-12", true)]
    #[case::viernes("2026-11-13", true)]
    #[case::sabado("2026-11-14", true)]
    #[case::domingo("2026-11-15", false)]
    fn sin_datos_se_muestra_de_lunes_a_sabado(#[case] dia: &str, #[case] visible: bool) {
        let dias = dias_visibles(fecha(LUNES), |_| false);
        assert_eq!(dias.contains(&fecha(dia)), visible);
    }

    #[test]
    fn el_domingo_se_muestra_solo_si_tiene_datos() {
        let dias = dias_visibles(fecha(LUNES), |d| d == 6);
        assert_eq!(dias.len(), 7);
        assert_eq!(dias.last(), Some(&fecha("2026-11-15")));
    }

    // ── Calendar edges ──────────────────────────────────────────────────────

    #[rstest]
    #[case::cambio_de_mes("2026-12-01", "2026-11-30")]
    #[case::cambio_de_anio("2027-01-01", "2026-12-28")]
    #[case::fin_de_anio("2026-12-31", "2026-12-28")]
    #[case::bisiesto_29_de_febrero("2028-02-29", "2028-02-28")]
    #[case::marzo_tras_bisiesto("2028-03-01", "2028-02-28")]
    fn el_lunes_de_la_semana_en_los_bordes_del_calendario(#[case] dia: &str, #[case] lunes: &str) {
        assert_eq!(lunes_de(fecha(dia)), fecha(lunes));
    }

    #[rstest]
    #[case::semana_anterior(LUNES, -7, "2026-11-02")]
    #[case::semana_siguiente(LUNES, 7, "2026-11-16")]
    #[case::cruza_mes("2026-11-30", 5, "2026-12-05")]
    #[case::cruza_anio("2026-12-28", 5, "2027-01-02")]
    #[case::cero_dias(LUNES, 0, LUNES)]
    fn sumar_dias_en_ambos_sentidos(
        #[case] desde: &str,
        #[case] dias: i64,
        #[case] esperado: &str,
    ) {
        assert_eq!(sumar_dias(fecha(desde), dias), fecha(esperado));
    }

    // ── Happy path: dates the app accepts ───────────────────────────────────

    #[rstest]
    #[case::formato_iso("2026-11-09", "2026-11-09")]
    #[case::espacios_alrededor(" 2026-11-09 ", "2026-11-09")]
    #[case::tabulador_y_salto_de_linea("\t2026-11-09\n", "2026-11-09")]
    #[case::sin_ceros_a_la_izquierda("2026-1-9", "2026-01-09")]
    #[case::bisiesto_valido("2028-02-29", "2028-02-29")]
    #[case::ultimo_dia_del_anio("2026-12-31", "2026-12-31")]
    fn fecha_valida_se_acepta(#[case] entrada: &str, #[case] esperado: &str) {
        assert_eq!(parse_o_hoy(Some(entrada)), fecha(esperado));
    }

    // ── Unhappy path: anything else falls back to today, never panics ───────

    #[rstest]
    #[case::ausente(None)]
    #[case::vacia(Some(""))]
    #[case::solo_espacios(Some("   "))]
    #[case::bisiesto_inexistente(Some("2026-02-29"))]
    #[case::mes_13(Some("2026-13-01"))]
    #[case::mes_cero(Some("2026-00-10"))]
    #[case::dia_31_de_noviembre(Some("2026-11-31"))]
    #[case::dia_cero(Some("2026-11-00"))]
    #[case::formato_dia_mes_anio(Some("09/11/2026"))]
    #[case::separador_barra(Some("2026/11/09"))]
    #[case::orden_invertido(Some("09-11-2026"))]
    #[case::con_hora(Some("2026-11-09T10:00"))]
    #[case::dia_en_ingles(Some("Monday"))]
    #[case::dia_en_espanol(Some("lunes"))]
    #[case::fecha_en_ingles(Some("Nov 9 2026"))]
    #[case::palabra_hoy(Some("hoy"))]
    #[case::html_script(Some("<script>alert(1)</script>"))]
    #[case::inyeccion_sql(Some("' OR 1=1 --"))]
    #[case::digitos_de_ancho_completo(Some("２０２６-11-09"))]
    #[case::caracter_nulo(Some("2026-11-09\0"))]
    #[case::emoji(Some("📅"))]
    #[case::anio_fuera_del_rango_de_chrono(Some("262143-12-31"))]
    #[case::anio_negativo_extremo(Some("-262144-01-01"))]
    fn fecha_invalida_usa_hoy(#[case] entrada: Option<&str>) {
        assert_eq!(parse_o_hoy(entrada), hoy());
    }

    #[test]
    fn texto_muy_largo_usa_hoy() {
        assert_eq!(parse_o_hoy(Some(&"9".repeat(10_000))), hoy());
    }

    // ── Week labels ─────────────────────────────────────────────────────────

    #[rstest]
    #[case::dentro_de_un_mes(LUNES, "9 Noviembre", "14 Noviembre, 2026")]
    #[case::cruza_mes("2026-11-30", "30 Noviembre", "5 Diciembre, 2026")]
    #[case::cruza_anio("2026-12-28", "28 Diciembre", "2 Enero, 2027")]
    fn rango_de_la_semana_de_lunes_a_sabado(
        #[case] lunes: &str,
        #[case] inicio: &str,
        #[case] fin: &str,
    ) {
        assert_eq!(
            rango_semana(fecha(lunes)),
            (inicio.to_owned(), fin.to_owned())
        );
    }

    #[rstest]
    #[case::dentro_de_un_mes(LUNES, "Semana del 9 al 14 de Noviembre")]
    #[case::cruza_mes("2026-11-30", "Semana del 30 de Noviembre al 5 de Diciembre")]
    #[case::cruza_anio("2026-12-28", "Semana del 28 de Diciembre al 2 de Enero")]
    fn la_etiqueta_muestra_ambos_meses_si_la_semana_cruza_de_mes(
        #[case] lunes: &str,
        #[case] esperado: &str,
    ) {
        assert_eq!(etiqueta_semana(fecha(lunes)), esperado);
    }

    #[rstest]
    #[case::dentro_de_un_mes(LUNES, "9-14/Noviembre/2026")]
    #[case::cruza_mes("2026-11-30", "30/Noviembre-5/Diciembre/2026")]
    #[case::cruza_anio("2026-12-28", "28/Diciembre/2026-2/Enero/2027")]
    fn el_periodo_del_comprobante_muestra_ambos_meses_si_la_semana_cruza_de_mes(
        #[case] lunes: &str,
        #[case] esperado: &str,
    ) {
        assert_eq!(periodo(fecha(lunes)), esperado);
    }
}
