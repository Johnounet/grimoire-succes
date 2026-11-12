//! Détection de l'installation Steam et des comptes présents sur le PC.

use serde::Serialize;
use std::path::{Path, PathBuf};

/// Écart entre un SteamID64 et l'ID de compte 32 bits (comptes individuels).
const STEAMID64_BASE: u64 = 76_561_197_960_265_728;

pub fn account_id_from_steamid64(id: u64) -> Option<u32> {
    id.checked_sub(STEAMID64_BASE).and_then(|v| u32::try_from(v).ok())
}

pub fn steamid64_from_account_id(id: u32) -> u64 {
    STEAMID64_BASE + id as u64
}

#[derive(Debug, Clone, Serialize)]
pub struct SteamAccount {
    pub account_id: u32,
    pub steamid64: u64,
    pub persona_name: Option<String>,
    pub most_recent: bool,
}

/// Chemins candidats, du plus probable au moins probable.
fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();

    #[cfg(windows)]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam") {
            if let Ok(p) = key.get_value::<String, _>("SteamPath") {
                out.push(PathBuf::from(p));
            }
        }
        out.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
        out.push(PathBuf::from(r"C:\Program Files\Steam"));
    }

    #[cfg(not(windows))]
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        out.push(home.join(".steam/steam"));
        out.push(home.join(".local/share/Steam"));
        out.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
        out.push(home.join(".steam/root"));
    }

    out
}

fn looks_like_steam(p: &Path) -> bool {
    p.join("appcache").is_dir() || p.join("steamapps").is_dir()
}

/// Trouve l'installation Steam. `custom` (réglage utilisateur) passe en premier.
pub fn find_steam(custom: Option<&Path>) -> Option<PathBuf> {
    custom
        .map(Path::to_path_buf)
        .into_iter()
        .chain(candidate_paths())
        .find(|p| looks_like_steam(p))
}

/// Comptes connus : `config/loginusers.vdf`, complété par les dossiers `userdata/`.
pub fn list_accounts(steam: &Path) -> Vec<SteamAccount> {
    let mut accounts: Vec<SteamAccount> = Vec::new();

    if let Ok(text) = std::fs::read_to_string(steam.join("config/loginusers.vdf")) {
        if let Ok(vdf) = keyvalues_parser::parse(&text).map(keyvalues_parser::Vdf::from) {
            if let Some(users) = vdf.value.get_obj() {
                for (id, entries) in users.iter() {
                    let Ok(steamid64) = id.parse::<u64>() else { continue };
                    let Some(account_id) = account_id_from_steamid64(steamid64) else { continue };
                    let obj = entries.first().and_then(|v| v.get_obj());
                    let field = |k: &str| {
                        obj.and_then(|o| o.get(k))
                            .and_then(|v| v.first())
                            .and_then(|v| v.get_str())
                            .map(str::to_string)
                    };
                    accounts.push(SteamAccount {
                        account_id,
                        steamid64,
                        persona_name: field("PersonaName"),
                        most_recent: field("MostRecent").as_deref() == Some("1"),
                    });
                }
            }
        }
    }

    if let Ok(dir) = std::fs::read_dir(steam.join("userdata")) {
        for e in dir.flatten() {
            let Some(account_id) = e.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            if account_id != 0 && !accounts.iter().any(|a| a.account_id == account_id) {
                accounts.push(SteamAccount {
                    account_id,
                    steamid64: steamid64_from_account_id(account_id),
                    persona_name: None,
                    most_recent: false,
                });
            }
        }
    }

    accounts.sort_by_key(|a| !a.most_recent);
    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_steamid() {
        assert_eq!(account_id_from_steamid64(76561198071247993), Some(110982265));
        assert_eq!(steamid64_from_account_id(110982265), 76561198071247993);
        assert_eq!(account_id_from_steamid64(42), None);
    }
}
