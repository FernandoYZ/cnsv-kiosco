pub mod envios;
pub mod handler;
pub mod repository;

use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct Pago {
    pub id: i64,
    pub monto: f64,
    pub fecha: NaiveDate,
}
