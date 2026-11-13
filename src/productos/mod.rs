pub mod handler;
pub mod repository;

#[derive(Debug, Clone)]
pub struct Producto {
    pub id: i64,
    pub nombre: String,
    pub precio_unitario: f64,
    pub esta_activo: bool,
}
