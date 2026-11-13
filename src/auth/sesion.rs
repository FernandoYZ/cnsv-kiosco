//! Stateless session token, same format as the Go app:
//! `base64url("id:puede_editar:expiry") + "." + base64url(HMAC-SHA256)`.

use std::sync::Arc;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub const COOKIE: &str = "kiosco_session";
/// 24 hours, as in Go.
pub const DURACION_SEGUNDOS: i64 = 86_400;

/// The logged-in user, available to handlers as an `Extension`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sesion {
    pub id_usuario: i64,
    pub puede_editar: bool,
}

/// HMAC key, random per process: restarting the server ends every session.
#[derive(Clone)]
pub struct Llave(Arc<[u8; 32]>);

impl Llave {
    pub fn aleatoria() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("operating system random generator");
        Self(Arc::new(bytes))
    }

    fn mac(&self) -> Hmac<Sha256> {
        Hmac::new_from_slice(self.0.as_slice()).expect("HMAC accepts keys of any length")
    }

    /// Token valid for [`DURACION_SEGUNDOS`] from `ahora` (Unix seconds).
    pub fn firmar(&self, sesion: Sesion, ahora: i64) -> String {
        let datos = format!(
            "{}:{}:{}",
            sesion.id_usuario,
            u8::from(sesion.puede_editar),
            ahora + DURACION_SEGUNDOS
        );
        let mut mac = self.mac();
        mac.update(datos.as_bytes());
        format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(&datos),
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        )
    }

    /// `None` when the token is malformed, tampered with or expired.
    pub fn verificar(&self, token: &str, ahora: i64) -> Option<Sesion> {
        let (datos, firma) = token.split_once('.')?;
        let datos = URL_SAFE_NO_PAD.decode(datos).ok()?;
        let firma = URL_SAFE_NO_PAD.decode(firma).ok()?;
        let mut mac = self.mac();
        mac.update(&datos);
        mac.verify_slice(&firma).ok()?;

        let datos = std::str::from_utf8(&datos).ok()?;
        let mut partes = datos.split(':');
        let id_usuario = partes.next()?.parse().ok()?;
        let puede_editar = match partes.next()? {
            "1" => true,
            "0" => false,
            _ => return None,
        };
        let expira: i64 = partes.next()?.parse().ok()?;
        if partes.next().is_some() || ahora >= expira {
            return None;
        }
        Some(Sesion {
            id_usuario,
            puede_editar,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DURACION_SEGUNDOS, Llave, Sesion};

    const AHORA: i64 = 1_800_000_000;
    const EDITOR: Sesion = Sesion {
        id_usuario: 7,
        puede_editar: true,
    };

    #[test]
    fn un_token_firmado_se_verifica_hasta_que_vence() {
        let llave = Llave::aleatoria();
        let token = llave.firmar(EDITOR, AHORA);
        assert_eq!(llave.verificar(&token, AHORA), Some(EDITOR));
        assert_eq!(
            llave.verificar(&token, AHORA + DURACION_SEGUNDOS - 1),
            Some(EDITOR)
        );
        assert_eq!(llave.verificar(&token, AHORA + DURACION_SEGUNDOS), None);
    }

    #[test]
    fn un_token_alterado_o_de_otra_llave_se_rechaza() {
        let llave = Llave::aleatoria();
        let token = llave.firmar(EDITOR, AHORA);
        assert_eq!(Llave::aleatoria().verificar(&token, AHORA), None, "restart");

        let (_, firma) = token.split_once('.').unwrap();
        let falso = format!("{}.{firma}", base64_url("7:1:9999999999"));
        assert_eq!(llave.verificar(&falso, AHORA), None, "forged payload");
        assert_eq!(llave.verificar("basura", AHORA), None);
        assert_eq!(llave.verificar("", AHORA), None);
    }

    fn base64_url(texto: &str) -> String {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        URL_SAFE_NO_PAD.encode(texto)
    }
}
