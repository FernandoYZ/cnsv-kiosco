//! Test fixtures: an in-memory SQLite with the exact production schema
//! (copied from `database/database.db`) and a small, known dataset.

use tokio_rusqlite::rusqlite::Connection;

use crate::{
    auth::{Auth, password},
    pagos::envios::EnviosRecientes,
    state::AppState,
};

/// Test users: `jenny` (id 1, can edit) and `kiosco` (id 2, consumption only).
pub const CLAVE: &str = "clave-de-prueba";

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
CREATE TABLE usuarios (
    id_usuario INTEGER PRIMARY KEY AUTOINCREMENT,
    usuario TEXT NOT NULL UNIQUE,
    contrasenha TEXT NOT NULL,
    puede_editar INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_consumos_estudiante_fecha ON consumos(id_estudiante, fecha_consumo);
CREATE INDEX idx_pagos_estudiante_fecha ON pagos(id_estudiante, fecha_pago);
CREATE INDEX idx_estudiantes_activo ON estudiantes(esta_activo) WHERE esta_activo = 1;
CREATE INDEX idx_estudiantes_apellido_nombre ON estudiantes(apellidos, nombres);
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

/// The real migration file, so tests run against the migrated schema.
pub const MIGRACION_001: &str = include_str!("../migrations/001_consumos_unique.sql");

/// Current production schema: legacy schema + migration 001 (UNIQUE cell index).
pub fn preparar(c: &Connection) {
    preparar_heredada(c);
    c.execute_batch(MIGRACION_001).expect("migration 001");
}

/// Schema as it was before migration 001 (duplicates allowed), for tests
/// that check the code still copes with a database copied from the VPS.
pub fn preparar_heredada(c: &Connection) {
    c.execute_batch(ESQUEMA).expect("schema");
    c.execute_batch(DATOS).expect("seed");
    let hash = password::generar(CLAVE, &[1; 16]);
    c.execute(
        "INSERT INTO usuarios VALUES (1, 'jenny', ?1, 1), (2, 'kiosco', ?1, 0)",
        [&hash],
    )
    .expect("users");
    c.pragma_update(None, "foreign_keys", "ON").expect("pragma");
}

pub fn conexion() -> Connection {
    let c = Connection::open_in_memory().expect("in-memory db");
    preparar(&c);
    c
}

pub fn conexion_heredada() -> Connection {
    let c = Connection::open_in_memory().expect("in-memory db");
    preparar_heredada(&c);
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
        auth: Auth::new(false),
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
