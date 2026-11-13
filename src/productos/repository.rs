use tokio_rusqlite::rusqlite::{self, Connection, OptionalExtension, Row, params};

use super::Producto;

fn mapear(row: &Row) -> rusqlite::Result<Producto> {
    Ok(Producto {
        id: row.get(0)?,
        nombre: row.get(1)?,
        precio_unitario: row.get(2)?,
        esta_activo: row.get(3)?,
    })
}

/// ObtenerTodosProductos: every product, active first.
pub fn todos(c: &Connection) -> rusqlite::Result<Vec<Producto>> {
    let mut stmt = c.prepare_cached(
        "SELECT id_producto, nombre, precio_unitario, esta_activo
         FROM productos
         ORDER BY esta_activo DESC, id_producto",
    )?;

    stmt.query_map([], mapear)?.collect()
}

/// ObtenerProductosActivos: products that can be sold.
pub fn activos(c: &Connection) -> rusqlite::Result<Vec<Producto>> {
    let mut stmt = c.prepare_cached(
        "SELECT id_producto, nombre, precio_unitario, esta_activo
         FROM productos
         WHERE esta_activo = 1
         ORDER BY id_producto",
    )?;

    stmt.query_map([], mapear)?.collect()
}

/// ObtenerProductoPorId.
pub fn por_id(c: &Connection, id: i64) -> rusqlite::Result<Option<Producto>> {
    c.query_row(
        "SELECT id_producto, nombre, precio_unitario, esta_activo
         FROM productos WHERE id_producto = ?1",
        [id],
        mapear,
    )
    .optional()
}

/// InsertarProducto: creates an active product.
pub fn insertar(c: &Connection, nombre: &str, precio: f64) -> rusqlite::Result<i64> {
    c.execute(
        "INSERT INTO productos (nombre, precio_unitario, esta_activo) VALUES (?1, ?2, 1)",
        params![nombre, precio],
    )?;
    Ok(c.last_insert_rowid())
}

/// ActualizarProducto: past consumption keeps its own `precio_unitario_venta`.
pub fn actualizar(c: &Connection, id: i64, nombre: &str, precio: f64) -> rusqlite::Result<usize> {
    c.execute(
        "UPDATE productos SET nombre = ?1, precio_unitario = ?2 WHERE id_producto = ?3",
        params![nombre, precio, id],
    )
}

/// CambiarEstadoProducto: logical enable/disable.
pub fn cambiar_estado(c: &Connection, id: i64, activo: bool) -> rusqlite::Result<usize> {
    c.execute(
        "UPDATE productos SET esta_activo = ?1 WHERE id_producto = ?2",
        params![activo, id],
    )
}
