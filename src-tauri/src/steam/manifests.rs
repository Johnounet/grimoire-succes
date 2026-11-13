//! Jeux installés, d'après les manifestes `steamapps/appmanifest_<appid>.acf` de chaque bibliothèque
//! Steam (`steamapps/libraryfolders.vdf`). Lecture seule.
//!
//! Le manifeste donne le nom officiel du jeu, même retiré du magasin (Atelier Ryza: Ever Darkness &
//! the Secret Hideout, remplacé par sa version DX), là où le schéma des succès n'a parfois qu'un nom
//! court ou un nom de code (« Atelier Ryza », « RINO »).

use keyvalues_parser::{Obj, Parser, Value, Vdf};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn child<'a>(obj: &'a Obj<'a>, key: &str) -> Option<&'a Value<'a>> {
    obj.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).and_then(|(_, v)| v.first())
}

fn parse(text: &str) -> Option<Vdf<'_>> {
    Parser::new()
        .parse(text)
        .ok()
        .or_else(|| Parser::new().literal_special_chars(true).parse(text).ok())
        .map(Into::into)
}

/// Dossiers `steamapps` de toutes les bibliothèques (celle de Steam d'abord).
fn library_dirs(steam: &Path) -> Vec<PathBuf> {
    let main = steam.join("steamapps");
    let mut dirs = vec![main.clone()];
    if let Ok(text) = std::fs::read_to_string(main.join("libraryfolders.vdf")) {
        if let Some(vdf) = parse(&text) {
            for (_, folder) in vdf.value.get_obj().map(|o| o.iter().collect::<Vec<_>>()).unwrap_or_default() {
                let Some(path) = folder.first().and_then(Value::get_obj).and_then(|f| child(f, "path")?.get_str()) else {
                    continue;
                };
                let dir = PathBuf::from(path.replace("\\\\", "\\")).join("steamapps");
                if !dirs.iter().any(|d| same_dir(d, &dir)) {
                    dirs.push(dir);
                }
            }
        }
    }
    dirs
}

fn same_dir(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase().replace('/', "\\") == b.to_string_lossy().to_lowercase().replace('/', "\\")
}

/// Nom d'un jeu d'après le texte de son manifeste.
pub fn parse_name(text: &str) -> Option<String> {
    let vdf = parse(text)?;
    let name = child(vdf.value.get_obj()?, "name")?.get_str()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Nom officiel des jeux installés, par AppID.
pub fn installed_names(steam: &Path) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    for dir in library_dirs(steam) {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let file = e.file_name();
            let Some(appid) = file
                .to_str()
                .and_then(|n| n.strip_prefix("appmanifest_"))
                .and_then(|n| n.strip_suffix(".acf"))
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            if let Some(name) = std::fs::read_to_string(e.path()).ok().as_deref().and_then(parse_name) {
                out.entry(appid).or_insert(name);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_le_nom_du_manifeste() {
        let text = "\"AppState\"\n{\n\t\"appid\"\t\t\"1121560\"\n\t\"name\"\t\t\"Atelier Ryza: Ever Darkness & the Secret Hideout\"\n\t\"installdir\"\t\t\"Atelier Ryza\"\n}\n";
        assert_eq!(parse_name(text).as_deref(), Some("Atelier Ryza: Ever Darkness & the Secret Hideout"));
        assert_eq!(parse_name("\"AppState\" { \"name\" \"\" }"), None);
    }
}
