use std::collections::HashMap;

use chrono::NaiveDate;
use tokio_rusqlite::rusqlite::{self, Connection, params};

use crate::estudiantes::Sector;

/// One stored row of a student × product × day cell.
pub struct FilaCelda {
    pub id: i64,
    pub cantidad: i64,
}

pub struct LineaResumen {
    pub id_estudiante: i64,
    pub nombres: String,
    pub apellidos: String,
    pub nombre_grado: String,
    pub nombre_producto: String,
    pub cantidad: i64,
}

pub struct ConsumoDetalle {
    pub fecha: NaiveDate,
    pub id_producto: i64,
    pub nombre_producto: String,
    pub cantidad: i64,
    pub total_linea: f64,
}

/// ActualizarConsumo step 1. Returns every row of the cell: legacy data has
/// duplicates because there is no UNIQUE constraint.
pub fn filas_de_celda(
    c: &Connection,
    id_estudiante: i64,
    id_producto: i64,
    fecha: NaiveDate,
) -> rusqlite::Result<Vec<FilaCelda>> {
    let mut stmt = c.prepare_cached(
        "SELECT id_consumo, cantidad FROM consumos
         WHERE id_estudiante = ?1 AND id_producto = ?2 AND fecha_consumo = ?3
         ORDER BY id_consumo",
    )?;

    stmt.query_map(params![id_estudiante, id_producto, fecha], |r| {
        Ok(FilaCelda {
            id: r.get(0)?,
            cantidad: r.get(1)?,
        })
    })?
    .collect()
}

/// RegistrarConsumo: `total_linea` is a generated column.
pub fn registrar(
    c: &Connection,
    id_estudiante: i64,
    id_producto: i64,
    cantidad: i64,
    precio: f64,
    fecha: NaiveDate,
) -> rusqlite::Result<()> {
    c.execute(
        "INSERT INTO consumos (id_estudiante, id_producto, cantidad, precio_unitario_venta, fecha_consumo)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id_estudiante, id_producto, cantidad, precio, fecha],
    )?;
    Ok(())
}

/// ActualizarConsumo step 2c.
pub fn actualizar(
    c: &Connection,
    id_consumo: i64,
    cantidad: i64,
    precio: f64,
) -> rusqlite::Result<()> {
    c.execute(
        "UPDATE consumos SET cantidad = ?1, precio_unitario_venta = ?2 WHERE id_consumo = ?3",
        params![cantidad, precio, id_consumo],
    )?;
    Ok(())
}

/// ActualizarConsumo step 2b.
pub fn eliminar(c: &Connection, id_consumo: i64) -> rusqlite::Result<()> {
    c.execute("DELETE FROM consumos WHERE id_consumo = ?1", [id_consumo])?;
    Ok(())
}

/// Quantities of one day keyed by (student, product), to prefill the grid.
pub fn cantidades_del_dia(
    c: &Connection,
    fecha: NaiveDate,
) -> rusqlite::Result<HashMap<(i64, i64), i64>> {
    let mut stmt = c.prepare_cached(
        "SELECT id_estudiante, id_producto, SUM(cantidad) FROM consumos
         WHERE fecha_consumo = ?1
         GROUP BY id_estudiante, id_producto",
    )?;

    stmt.query_map([fecha], |r| Ok(((r.get(0)?, r.get(1)?), r.get(2)?)))?
        .collect()
}

/// ObtenerResumenDiario.
pub fn resumen_diario(
    c: &Connection,
    sector: Sector,
    fecha: NaiveDate,
) -> rusqlite::Result<Vec<LineaResumen>> {
    let sql = format!(
        "SELECT e.id_estudiante, e.nombres, e.apellidos,
                g.anio_grado || ' ' || g.nivel_grado AS nombre_grado,
                p.nombre AS nombre_producto,
                c.cantidad
         FROM consumos c
         JOIN estudiantes e ON c.id_estudiante = e.id_estudiante
         JOIN grados g      ON e.id_grado = g.id_grado
         JOIN productos p   ON c.id_producto = p.id_producto
         WHERE c.fecha_consumo = ?1
           AND e.id_grado IN ({})
         ORDER BY e.apellidos, e.nombres, p.nombre",
        sector.grados_sql()
    );
    let mut stmt = c.prepare_cached(&sql)?;

    stmt.query_map([fecha], |r| {
        Ok(LineaResumen {
            id_estudiante: r.get(0)?,
            nombres: r.get(1)?,
            apellidos: r.get(2)?,
            nombre_grado: r.get(3)?,
            nombre_producto: r.get(4)?,
            cantidad: r.get(5)?,
        })
    })?
    .collect()
}

/// Consumption of one student between two dates (inclusive), with product names.
pub fn de_estudiante_entre(
    c: &Connection,
    id_estudiante: i64,
    inicio: NaiveDate,
    fin: NaiveDate,
) -> rusqlite::Result<Vec<ConsumoDetalle>> {
    let mut stmt = c.prepare_cached(
        "SELECT c.fecha_consumo, c.id_producto, p.nombre, c.cantidad, c.total_linea
         FROM consumos c
         JOIN productos p ON p.id_producto = c.id_producto
         WHERE c.id_estudiante = ?1 AND c.fecha_consumo BETWEEN ?2 AND ?3
         ORDER BY p.nombre, c.fecha_consumo",
    )?;

    stmt.query_map(params![id_estudiante, inicio, fin], |r| {
        Ok(ConsumoDetalle {
            fecha: r.get(0)?,
            id_producto: r.get(1)?,
            nombre_producto: r.get(2)?,
            cantidad: r.get(3)?,
            total_linea: r.get(4)?,
        })
    })?
    .collect()
}

/// ObtenerDeudaAnterior, consumption part.
pub fn total_antes_de(
    c: &Connection,
    id_estudiante: i64,
    fecha: NaiveDate,
) -> rusqlite::Result<f64> {
    c.query_row(
        "SELECT COALESCE(SUM(total_linea), 0) FROM consumos
         WHERE id_estudiante = ?1 AND fecha_consumo < ?2",
        params![id_estudiante, fecha],
        |r| r.get(0),
    )
}
