use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use tokio_rusqlite::rusqlite::{self, Connection};

use super::repository;
use crate::{
    estudiantes::{self, Sector},
    productos,
};

pub struct Celda {
    pub id_estudiante: i64,
    pub id_producto: i64,
    pub cantidad: i64,
}

#[derive(Default)]
pub struct ResultadoRegistro {
    pub cambios: usize,
    pub ignoradas: usize,
}

/// Saves the daily grid in a single transaction.
///
/// Only cells whose quantity changed are written, so untouched cells keep the
/// price they were sold at. Cells of inactive products or of students outside
/// the sector are ignored and counted.
pub fn guardar_registro(
    c: &mut Connection,
    sector: Sector,
    fecha: NaiveDate,
    celdas: &[Celda],
) -> rusqlite::Result<ResultadoRegistro> {
    let tx = c.transaction()?;

    let estudiantes: HashSet<i64> = estudiantes::repository::activos_por_sector(&tx, sector)?
        .into_iter()
        .map(|e| e.id)
        .collect();
    let precios: HashMap<i64, f64> = productos::repository::activos(&tx)?
        .into_iter()
        .map(|p| (p.id, p.precio_unitario))
        .collect();

    let mut resultado = ResultadoRegistro::default();
    for celda in celdas {
        let precio = match precios.get(&celda.id_producto) {
            Some(precio) if estudiantes.contains(&celda.id_estudiante) => *precio,
            _ => {
                resultado.ignoradas += 1;
                continue;
            }
        };
        if actualizar_celda(&tx, celda, fecha, precio)? {
            resultado.cambios += 1;
        }
    }

    tx.commit()?;
    Ok(resultado)
}

/// `ActualizarConsumo`, now transactional. Returns whether the cell changed.
///
/// If legacy duplicate rows exist for the cell, they are merged into the first
/// row when the cell is edited.
fn actualizar_celda(
    tx: &Connection,
    celda: &Celda,
    fecha: NaiveDate,
    precio: f64,
) -> rusqlite::Result<bool> {
    let filas = repository::filas_de_celda(tx, celda.id_estudiante, celda.id_producto, fecha)?;
    let actual: i64 = filas.iter().map(|f| f.cantidad).sum();
    if actual == celda.cantidad {
        return Ok(false);
    }

    match filas.split_first() {
        None => repository::registrar(
            tx,
            celda.id_estudiante,
            celda.id_producto,
            celda.cantidad,
            precio,
            fecha,
        )?,
        Some((primera, duplicadas)) => {
            for fila in duplicadas {
                repository::eliminar(tx, fila.id)?;
            }
            if celda.cantidad <= 0 {
                repository::eliminar(tx, primera.id)?;
            } else {
                repository::actualizar(tx, primera.id, celda.cantidad, precio)?;
            }
        }
    }
    Ok(true)
}
