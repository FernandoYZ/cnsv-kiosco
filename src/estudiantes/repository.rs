use tokio_rusqlite::rusqlite::{self, Connection, OptionalExtension, Row, params};

use super::{Estudiante, Sector};

const SELECT_ESTUDIANTE: &str =
    "SELECT e.id_estudiante, e.nombres, e.apellidos, e.id_grado, e.esta_activo,
       g.anio_grado || ' ' || g.nivel_grado AS nombre_grado
FROM estudiantes e
LEFT JOIN grados g ON e.id_grado = g.id_grado";

fn mapear(row: &Row) -> rusqlite::Result<Estudiante> {
    Ok(Estudiante {
        id: row.get(0)?,
        nombres: row.get(1)?,
        apellidos: row.get(2)?,
        id_grado: row.get(3)?,
        esta_activo: row.get(4)?,
        nombre_grado: row.get(5)?,
    })
}

/// ObtenerEstudiantesPorGrado / ObtenerEstudiantesActivos (`id_grado = 0` → all grades).
pub fn activos_por_grado(c: &Connection, id_grado: i64) -> rusqlite::Result<Vec<Estudiante>> {
    let sql = format!(
        "{SELECT_ESTUDIANTE}
         WHERE e.esta_activo = 1 AND (?1 = 0 OR e.id_grado = ?1)
         ORDER BY g.nivel_grado, g.nombre_grado, e.apellidos, e.nombres"
    );
    let mut stmt = c.prepare_cached(&sql)?;

    stmt.query_map([id_grado], mapear)?.collect()
}

/// ObtenerTodosEstudiantes: active first.
pub fn todos(c: &Connection) -> rusqlite::Result<Vec<Estudiante>> {
    let sql = format!(
        "{SELECT_ESTUDIANTE}
         ORDER BY e.esta_activo DESC, g.nivel_grado, g.nombre_grado, e.apellidos, e.nombres"
    );
    let mut stmt = c.prepare_cached(&sql)?;

    stmt.query_map([], mapear)?.collect()
}

/// ObtenerEstudiantePorId.
pub fn por_id(c: &Connection, id: i64) -> rusqlite::Result<Option<Estudiante>> {
    let sql = format!("{SELECT_ESTUDIANTE} WHERE e.id_estudiante = ?1");
    c.query_row(&sql, [id], mapear).optional()
}

/// ObtenerEstudiantesActivosPorSector.
pub fn activos_por_sector(c: &Connection, sector: Sector) -> rusqlite::Result<Vec<Estudiante>> {
    let sql = format!(
        "{SELECT_ESTUDIANTE}
         WHERE e.esta_activo = 1 AND e.id_grado IN ({})
         ORDER BY e.apellidos, e.nombres",
        sector.grados_sql()
    );
    let mut stmt = c.prepare_cached(&sql)?;

    stmt.query_map([], mapear)?.collect()
}

/// InsertarEstudiante: new students are active.
pub fn insertar(
    c: &Connection,
    nombres: &str,
    apellidos: &str,
    id_grado: i64,
) -> rusqlite::Result<i64> {
    c.execute(
        "INSERT INTO estudiantes (nombres, apellidos, id_grado, esta_activo) VALUES (?1, ?2, ?3, 1)",
        params![nombres, apellidos, id_grado],
    )?;
    Ok(c.last_insert_rowid())
}

/// ActualizarEstudiante.
pub fn actualizar(
    c: &Connection,
    id: i64,
    nombres: &str,
    apellidos: &str,
    id_grado: i64,
) -> rusqlite::Result<usize> {
    c.execute(
        "UPDATE estudiantes SET nombres = ?1, apellidos = ?2, id_grado = ?3 WHERE id_estudiante = ?4",
        params![nombres, apellidos, id_grado, id],
    )
}

/// CambiarEstadoEstudiante: logical enable/disable.
pub fn cambiar_estado(c: &Connection, id: i64, activo: bool) -> rusqlite::Result<usize> {
    c.execute(
        "UPDATE estudiantes SET esta_activo = ?1 WHERE id_estudiante = ?2",
        params![activo, id],
    )
}
