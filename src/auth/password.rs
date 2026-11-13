//! argon2id verification of the `usuarios.contrasenha` PHC strings created
//! by the Go app (`$argon2id$v=19$m=..,t=..,p=..$salt$hash`).

use std::sync::LazyLock;

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};

/// Memory, iterations, lanes and output bytes of the existing hashes.
const PARAMETROS: (u32, u32, u32, usize) = (16, 3, 1, 16);

/// Parameters, salt length and output length are read from `hash`.
pub fn verificar(clave: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(clave.as_bytes(), &hash)
            .is_ok()
    })
}

/// PHC string with the same parameters as the existing users.
pub fn generar(clave: &str, sal: &[u8; 16]) -> String {
    let (m, t, p, largo) = PARAMETROS;
    let params = Params::new(m, t, p, Some(largo)).expect("valid argon2 parameters");
    let sal = SaltString::encode_b64(sal).expect("16-byte salt");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password(clave.as_bytes(), &sal)
        .expect("argon2 hashing")
        .to_string()
}

/// Checked against when the user does not exist, so both failures cost the same.
pub static HASH_SEÑUELO: LazyLock<String> = LazyLock::new(|| generar("", &[0; 16]));

#[cfg(test)]
mod tests {
    use super::{generar, verificar};

    #[test]
    fn verifica_hashes_con_los_parametros_de_los_usuarios_existentes() {
        let hash = generar("secreta", &[7; 16]);
        assert!(hash.starts_with("$argon2id$v=19$m=16,t=3,p=1$"), "{hash}");
        // 16-byte salt and output → 22 base64 characters each, like the Go hashes.
        let partes: Vec<&str> = hash.split('$').collect();
        assert_eq!((partes[4].len(), partes[5].len()), (22, 22));

        assert!(verificar("secreta", &hash));
        assert!(!verificar("Secreta", &hash));
        assert!(!verificar("", &hash));
    }

    #[test]
    fn un_hash_mal_formado_nunca_valida() {
        assert!(!verificar("secreta", "secreta"));
        assert!(!verificar("", ""));
    }
}
