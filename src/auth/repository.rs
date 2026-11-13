use tokio_rusqlite::rusqlite::{self, Connection, OptionalExtension};

pub struct Usuario {
    pub id: i64,
    pub contrasenha: String,
    pub puede_editar: bool,
}

pub fn por_nombre(c: &Connection, usuario: &str) -> rusqlite::Result<Option<Usuario>> {
    c.query_row(
        "SELECT id_usuario, contrasenha, puede_editar FROM usuarios WHERE usuario = ?1",
        [usuario],
        |r| {
            Ok(Usuario {
                id: r.get(0)?,
                contrasenha: r.get(1)?,
                puede_editar: r.get(2)?,
            })
        },
    )
    .optional()
}
