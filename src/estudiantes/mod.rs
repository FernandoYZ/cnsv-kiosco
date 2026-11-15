use std::ops::RangeInclusive;

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

fn grado_valido(id: i64) -> bool {
    GRADOS.iter().any(|g| g.id == id)
}

/// Parses a grade id from a query string; `None` when missing or unknown.
pub fn grado_desde(valor: Option<&str>) -> Option<i64> {
    valor
        .and_then(|v| v.trim().parse().ok())
        .filter(|id| grado_valido(*id))
}

/// Kiosk sectors: `menor` = grades 1–4, `mayor` = grades 5–7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sector {
    Menor,
    Mayor,
}

impl Sector {
    pub fn desde(valor: &str) -> Option<Self> {
        match valor {
            "menor" => Some(Sector::Menor),
            "mayor" => Some(Sector::Mayor),
            _ => None,
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
            Sector::Menor => "Sector Menor",
            Sector::Mayor => "Sector Mayor",
        }
    }

    fn ids(self) -> RangeInclusive<i64> {
        match self {
            Sector::Menor => 1..=4,
            Sector::Mayor => 5..=7,
        }
    }

    /// Sector that groups a grade (`None` for an unknown grade).
    pub fn de_grado(id_grado: i64) -> Option<Self> {
        [Sector::Menor, Sector::Mayor]
            .into_iter()
            .find(|s| s.ids().contains(&id_grado))
    }

    pub fn grados(self) -> impl Iterator<Item = &'static Grado> {
        let ids = self.ids();
        GRADOS.iter().filter(move |g| ids.contains(&g.id))
    }

    /// SQL condition on `e.id_grado`, built from constants, never from user input.
    pub fn condicion_sql(self) -> String {
        let ids = self.ids();
        format!("e.id_grado BETWEEN {} AND {}", ids.start(), ids.end())
    }
}
