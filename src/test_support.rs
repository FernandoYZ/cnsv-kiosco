//! Test fixtures: an in-memory SQLite with the exact production schema
//! (copied from `database/database.db`) and a small, known dataset.

use tokio_rusqlite::rusqlite::Connection;

use crate::{pagos::envios::EnviosRecientes, state::AppState};

/// Production schema, verbatim from `sqlite_master`.
pub const ESQUEMA: &str = r#"
CREATE TABLE grados (
    id_grado INTEGER PRIMARY KEY AUTOINCREMENT,
    nivel_grado TEXT NOT NULL,
    nombre_grado TEXT NOT NULL,
    anio_grado TEXT NOT NULL
);
CREATE TABLE estudiantes (
    id_estudiante INTEGER PRIMARY KEY AUTOINCREMENT,
    nombres TEXT NOT NULL,
    apellidos TEXT NOT NULL,
    id_grado INTEGER,
    esta_activo INTEGER DEFAULT 1 NOT NULL,
    FOREIGN KEY (id_grado) REFERENCES grados(id_grado)
);
CREATE TABLE productos (
    id_producto INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre TEXT NOT NULL,
    precio_unitario NUMERIC(10, 2) NOT NULL,
    esta_activo INTEGER DEFAULT 1 NOT NULL
);
CREATE TABLE consumos (
    id_consumo INTEGER PRIMARY KEY AUTOINCREMENT,
    id_estudiante INTEGER NOT NULL,
    id_producto INTEGER NOT NULL,
    cantidad INTEGER NOT NULL DEFAULT 1,
    precio_unitario_venta NUMERIC(10, 2) NOT NULL,
    total_linea NUMERIC(10, 2) GENERATED ALWAYS AS (cantidad * precio_unitario_venta) STORED,
    fecha_consumo DATE NOT NULL,
    FOREIGN KEY (id_estudiante) REFERENCES estudiantes(id_estudiante),
    FOREIGN KEY (id_producto) REFERENCES productos(id_producto)
);
CREATE TABLE pagos (
    id_pago INTEGER PRIMARY KEY AUTOINCREMENT,
    id_estudiante INTEGER NOT NULL,
    monto NUMERIC(10, 2) NOT NULL,
    fecha_pago DATE NOT NULL,
    FOREIGN KEY (id_estudiante) REFERENCES estudiantes(id_estudiante)
);
CREATE INDEX idx_consumos_estudiante_fecha ON consumos(id_estudiante, fecha_consumo);
CREATE INDEX idx_pagos_estudiante_fecha ON pagos(id_estudiante, fecha_pago);
"#;

/// Known dataset:
/// - products: 1 Gelatina 1.50 (active), 2 Broaster 5.00 (active), 3 Antiguo 2.00 (inactive)
/// - students: 1 Ana (grade 3, active), 2 Luis (grade 7, active), 3 Baja (grade 3, inactive)
const DATOS: &str = r#"
INSERT INTO grados VALUES
  (1,'Primaria','Quinto','5to'), (2,'Primaria','Sexto','6to'), (3,'Secundaria','Primero','1ro'),
  (4,'Secundaria','Segundo','2do'), (5,'Secundaria','Tercero','3ro'), (6,'Secundaria','Cuarto','4to'),
  (7,'Secundaria','Quinto','5to');
INSERT INTO productos VALUES (1,'Gelatina',1.5,1), (2,'Broaster',5,1), (3,'Antiguo',2,0);
INSERT INTO estudiantes VALUES (1,'Ana','Alva',3,1), (2,'Luis','Bravo',7,1), (3,'Rosa','Baja',3,0);
"#;

pub fn preparar(c: &Connection) {
    c.execute_batch(ESQUEMA).expect("schema");
    c.execute_batch(DATOS).expect("seed");
    c.pragma_update(None, "foreign_keys", "ON").expect("pragma");
}

pub fn conexion() -> Connection {
    let c = Connection::open_in_memory().expect("in-memory db");
    preparar(&c);
    c
}

pub async fn estado() -> AppState {
    let db = tokio_rusqlite::Connection::open_in_memory()
        .await
        .expect("in-memory db");
    db.call(|c| {
        preparar(c);
        Ok::<_, tokio_rusqlite::rusqlite::Error>(())
    })
    .await
    .expect("prepare");
    AppState {
        db,
        envios_pago: EnviosRecientes::default(),
    }
}

/// Runs a query returning one value, for assertions.
pub async fn valor<T>(estado: &AppState, sql: &'static str) -> T
where
    T: tokio_rusqlite::rusqlite::types::FromSql + Send + 'static,
{
    estado
        .db
        .call(move |c| c.query_row(sql, [], |r| r.get::<_, T>(0)))
        .await
        .expect("query")
}
