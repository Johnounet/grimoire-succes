//! Lecteur du format KeyValues binaire de Valve, utilisé par les fichiers
//! `appcache/stats/*.bin`. Format non documenté officiellement :
//! une suite d'entrées `<octet de type><clé terminée par \0><valeur>`.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Kv {
    Node(Vec<(String, Kv)>),
    Str(String),
    Int(i32),
    Float(f32),
    UInt64(u64),
    Int64(i64),
    /// Types rares (pointeur, couleur, chaîne large) : conservés bruts.
    Other(Vec<u8>),
}

#[derive(Debug)]
pub struct KvError(pub String);

impl fmt::Display for KvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KeyValues binaire invalide : {}", self.0)
    }
}

impl std::error::Error for KvError {}

const T_NODE: u8 = 0x00;
const T_STRING: u8 = 0x01;
const T_INT32: u8 = 0x02;
const T_FLOAT32: u8 = 0x03;
const T_POINTER: u8 = 0x04;
const T_WSTRING: u8 = 0x05;
const T_COLOR: u8 = 0x06;
const T_UINT64: u8 = 0x07;
const T_END: u8 = 0x08;
const T_INT64: u8 = 0x0A;
const T_END_ALT: u8 = 0x0B;

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<Option<u8>, KvError> {
        Ok(self.buf.get(self.pos).copied().inspect(|_| self.pos += 1))
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], KvError> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.buf.len());
        let end = end.ok_or_else(|| KvError(format!("fin de fichier inattendue à l'octet {}", self.pos)))?;
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn cstr(&mut self) -> Result<String, KvError> {
        let rest = &self.buf[self.pos..];
        let len = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| KvError(format!("chaîne non terminée à l'octet {}", self.pos)))?;
        let s = String::from_utf8_lossy(&rest[..len]).into_owned();
        self.pos += len + 1;
        Ok(s)
    }

    /// Lit des entrées jusqu'à un marqueur de fin ou la fin du fichier.
    fn entries(&mut self, top_level: bool) -> Result<Vec<(String, Kv)>, KvError> {
        let mut out = Vec::new();
        loop {
            let Some(t) = self.byte()? else {
                if top_level {
                    return Ok(out);
                }
                return Err(KvError("nœud non fermé".into()));
            };
            if t == T_END || t == T_END_ALT {
                return Ok(out);
            }
            let key = self.cstr()?;
            let val = match t {
                T_NODE => Kv::Node(self.entries(false)?),
                T_STRING => Kv::Str(self.cstr()?),
                T_INT32 => Kv::Int(i32::from_le_bytes(self.take(4)?.try_into().unwrap())),
                T_FLOAT32 => Kv::Float(f32::from_le_bytes(self.take(4)?.try_into().unwrap())),
                T_UINT64 => Kv::UInt64(u64::from_le_bytes(self.take(8)?.try_into().unwrap())),
                T_INT64 => Kv::Int64(i64::from_le_bytes(self.take(8)?.try_into().unwrap())),
                T_POINTER | T_COLOR => Kv::Other(self.take(4)?.to_vec()),
                T_WSTRING => {
                    // UTF-16 terminée par deux octets nuls
                    let start = self.pos;
                    while self.take(2)? != [0, 0] {}
                    Kv::Other(self.buf[start..self.pos - 2].to_vec())
                }
                other => {
                    return Err(KvError(format!("type inconnu 0x{other:02x} à l'octet {}", self.pos)))
                }
            };
            out.push((key, val));
        }
    }
}

/// Analyse un fichier entier. Renvoie le nœud racine (liste des entrées de premier niveau).
pub fn parse(buf: &[u8]) -> Result<Kv, KvError> {
    let mut r = Reader { buf, pos: 0 };
    Ok(Kv::Node(r.entries(true)?))
}

impl Kv {
    /// Enfant par clé, insensible à la casse (Valve n'est pas constant).
    pub fn get(&self, key: &str) -> Option<&Kv> {
        match self {
            Kv::Node(items) => items.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn children(&self) -> &[(String, Kv)] {
        match self {
            Kv::Node(items) => items,
            _ => &[],
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Kv::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Valeur entière, que Valve l'ait stockée en nombre ou en texte.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Kv::Int(v) => Some(*v as i64),
            Kv::Int64(v) => Some(*v),
            Kv::UInt64(v) => i64::try_from(*v).ok(),
            Kv::Float(v) => Some(*v as i64),
            Kv::Str(s) => s.trim().parse().ok(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.as_i64().map(|v| v != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_un_arbre_simple() {
        let mut b = vec![T_NODE];
        b.extend(b"620\0");
        b.push(T_STRING);
        b.extend(b"gamename\0Portal 2\0");
        b.push(T_INT32);
        b.extend(b"version\0");
        b.extend(42i32.to_le_bytes());
        b.push(T_END);
        b.push(T_END);

        let root = parse(&b).unwrap();
        let game = root.get("620").unwrap();
        assert_eq!(game.get("gamename").unwrap().as_str(), Some("Portal 2"));
        assert_eq!(game.get("VERSION").unwrap().as_i64(), Some(42));
    }

    #[test]
    fn refuse_un_fichier_tronque() {
        let b = [T_NODE, b'a', 0, T_INT32, b'x', 0, 1];
        assert!(parse(&b).is_err());
    }
}
