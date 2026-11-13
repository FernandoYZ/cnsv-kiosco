//! URLs linked from more than one screen, so handlers do not import each other.

use chrono::NaiveDate;

use crate::fechas;

/// Appends `&grado=N` to a URL that already carries a query string.
pub fn con_grado(url: String, grado: Option<i64>) -> String {
    match grado {
        Some(g) => format!("{url}&grado={g}"),
        None => url,
    }
}

/// Weekly table (`/`) of the week containing `fecha`.
pub fn semana(fecha: NaiveDate, grado: Option<i64>) -> String {
    con_grado(format!("/?fecha={fecha}"), grado)
}

/// Payments of one student for the week containing `fecha`.
pub fn pagos(id_estudiante: i64, fecha: NaiveDate, grado: Option<i64>) -> String {
    con_grado(
        format!("/pagos/{id_estudiante}?fecha={}", fechas::lunes_de(fecha)),
        grado,
    )
}

/// Weekly receipt of one student.
pub fn comprobante(id_estudiante: i64, lunes: NaiveDate, grado: Option<i64>) -> String {
    con_grado(format!("/comprobante/{id_estudiante}?fecha={lunes}"), grado)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::{comprobante, pagos, semana};

    #[test]
    fn el_grado_solo_se_agrega_cuando_existe() {
        let jueves = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let lunes = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        assert_eq!(semana(jueves, Some(3)), "/?fecha=2026-10-29&grado=3");
        assert_eq!(semana(jueves, None), "/?fecha=2026-10-29");
        assert_eq!(
            pagos(1, jueves, Some(3)),
            "/pagos/1?fecha=2026-10-26&grado=3"
        );
        assert_eq!(
            comprobante(1, lunes, None),
            "/comprobante/1?fecha=2026-10-26"
        );
    }
}
