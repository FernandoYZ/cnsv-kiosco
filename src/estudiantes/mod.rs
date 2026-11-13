pub mod handler;
pub mod repository;

#[derive(Debug, Clone)]
pub struct Estudiante {
    pub id: i64,
    pub nombres: String,
    pub apellidos: String,
    pub id_grado: Option<i64>,
    pub esta_activo: bool,
    pub nombre_grado: Option<String>,
}

impl Estudiante {
    pub fn nombre_completo(&self) -> String {
        format!("{}, {}", self.apellidos, self.nombres)
    }

    pub fn grado(&self) -> &str {
        self.nombre_grado.as_deref().unwrap_or("Sin grado")
    }
}

pub struct Grado {
    pub id: i64,
    pub nombre: &'static str,
}

/// Static grade catalog. Mirrors the `grados` table (ids 1–7), as the Go
/// application does with `ObtenerGradosEstaticos()`.
pub const GRADOS: [Grado; 7] = [
    Grado {
        id: 1,
        nombre: "5to Primaria",
    },
    Grado {
        id: 2,
        nombre: "6to Primaria",
    },
    Grado {
        id: 3,
        nombre: "1ro Secundaria",
    },
    Grado {
        id: 4,
        nombre: "2do Secundaria",
    },
    Grado {
        id: 5,
        nombre: "3ro Secundaria",
    },
    Grado {
        id: 6,
        nombre: "4to Secundaria",
    },
    Grado {
        id: 7,
        nombre: "5to Secundaria",
    },
];

pub fn grado_valido(id: i64) -> bool {
    GRADOS.iter().any(|g| g.id == id)
}

/// Kiosk sectors: `menor` = grades 1–4, `mayor` = grades 5–7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sector {
    Menor,
    Mayor,
}

impl Sector {
    pub const TODOS: [Sector; 2] = [Sector::Menor, Sector::Mayor];

    /// Unknown or missing values default to `menor`.
    pub fn desde(valor: Option<&str>) -> Self {
        match valor {
            Some("mayor") => Sector::Mayor,
            _ => Sector::Menor,
        }
    }

    pub fn clave(self) -> &'static str {
        match self {
            Sector::Menor => "menor",
            Sector::Mayor => "mayor",
        }
    }

    pub fn nombre(self) -> &'static str {
        match self {
            Sector::Menor => "Sector menor (5to Prim. – 2do Sec.)",
            Sector::Mayor => "Sector mayor (3ro – 5to Sec.)",
        }
    }

    /// SQL `IN` list built from constants, never from user input.
    pub fn grados_sql(self) -> &'static str {
        match self {
            Sector::Menor => "1,2,3,4",
            Sector::Mayor => "5,6,7",
        }
    }
}
