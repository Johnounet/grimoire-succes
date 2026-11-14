//! Magasin Steam : recherche de jeux, nom officiel, jaquettes.

use super::web::{get_bytes, get_text};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// Ce que le joueur a tapé : un jeu précis (AppID ou lien) ou un nom à chercher.
#[derive(Debug, PartialEq)]
pub enum Query {
    AppId(u32),
    Name(String),
}

/// Reconnaît un AppID (`1245620`) ou un lien Steam :
/// `store.steampowered.com/app/<id>`, `steamcommunity.com/stats/<id>`,
/// `steam://run/<id>`, `steam://rungameid/<id>`, `steam://store/<id>`…
pub fn parse_query(input: &str) -> Query {
    let s = input.trim();
    if let Ok(id) = s.parse::<u32>() {
        return Query::AppId(id);
    }
    const MARKERS: &[&str] = &[
        "/app/",
        "/stats/",
        "/achievements/",
        "steam://run/",
        "steam://rungameid/",
        "steam://store/",
        "steam://install/",
        "steam://launch/",
        "/details/",
    ];
    let lower = s.to_ascii_lowercase();
    if lower.contains("steampowered.com") || lower.contains("steamcommunity.com") || lower.starts_with("steam://") {
        for m in MARKERS {
            if let Some(i) = lower.find(m) {
                let digits: String = s[i + m.len()..].chars().take_while(char::is_ascii_digit).collect();
                if let Ok(id) = digits.parse::<u32>() {
                    return Query::AppId(id);
                }
            }
        }
    }
    Query::Name(s.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct StoreHit {
    pub appid: u32,
    pub name: String,
}

/// Recherche par nom dans le magasin (sans clé).
/// `country` (code ISO, ex. `FR`) est indispensable : sans lui, Steam renvoie souvent une liste vide.
pub async fn search(term: &str, lang: &str, country: &str) -> Result<Vec<StoreHit>, String> {
    let url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&l={lang}&cc={}",
        urlencode(term),
        urlencode(country)
    );
    let v: Value = serde_json::from_str(&get_text(&url, lang).await?).map_err(|e| e.to_string())?;
    Ok(v["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|i| i["type"] == "app")
        .filter_map(|i| {
            Some(StoreHit {
                appid: u32::try_from(i["id"].as_u64()?).ok()?,
                name: i["name"].as_str()?.to_string(),
            })
        })
        .collect())
}

/// Nom officiel d'un jeu. `Ok(None)` si Steam ne connaît pas cet AppID.
pub async fn app_name(appid: u32, lang: &str) -> Result<Option<String>, String> {
    let url = format!("https://store.steampowered.com/api/appdetails?appids={appid}&filters=basic&l={lang}");
    let v: Value = serde_json::from_str(&get_text(&url, lang).await?).map_err(|e| e.to_string())?;
    let entry = &v[appid.to_string()];
    if entry["success"] != true {
        return Ok(None);
    }
    Ok(entry["data"]["name"].as_str().map(str::to_string))
}

const CDN: &str = "https://shared.fastly.steamstatic.com/store_item_assets/";

/// Ancienne adresse fixe d'une image : seulement en secours (voir [`assets`]).
fn cdn(appid: u32, file: &str) -> String {
    format!("{CDN}steam/apps/{appid}/{file}")
}

/// Petite image horizontale, pour les listes de résultats (adresse fixe, voir [`assets`]).
pub fn capsule_url(appid: u32) -> String {
    cdn(appid, "capsule_231x87.jpg")
}

/// Adresses exactes des images d'un jeu.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Assets {
    /// Jaquette verticale 600×900
    pub portrait: Option<String>,
    pub header: Option<String>,
    /// Petite image horizontale 231×87
    pub small_capsule: Option<String>,
}

/// Adresses des images de plusieurs jeux, d'après le magasin.
///
/// Les images des jeux récents (ou dont l'éditeur a changé les images) sont rangées dans un
/// sous-dossier à code (`apps/<id>/<code>/header.jpg`) : l'adresse fixe répond 404.
/// `IStoreBrowseService/GetItems` (sans clé) donne le chemin exact ; `country_code` est
/// indispensable, sans lui la réponse est vide. Un jeu inconnu est simplement absent du résultat.
pub async fn assets(appids: &[u32], country: &str) -> Result<HashMap<u32, Assets>, String> {
    let mut out = HashMap::new();
    for chunk in appids.chunks(100) {
        let input = serde_json::json!({
            "ids": chunk.iter().map(|id| serde_json::json!({ "appid": id })).collect::<Vec<_>>(),
            "context": { "country_code": country },
            "data_request": { "include_assets": true },
        });
        let url = format!(
            "https://api.steampowered.com/IStoreBrowseService/GetItems/v1/?input_json={}",
            urlencode(&input.to_string())
        );
        let v: Value = serde_json::from_str(&get_text(&url, "english").await?).map_err(|e| e.to_string())?;
        out.extend(parse_assets(&v));
    }
    Ok(out)
}

fn parse_assets(v: &Value) -> HashMap<u32, Assets> {
    v.pointer("/response/store_items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|i| i["success"] == 1)
        .filter_map(|i| {
            let appid = u32::try_from(i["appid"].as_u64()?).ok()?;
            let a = &i["assets"];
            // Ex. « steam/apps/620/${FILENAME}?t=1790187113 »
            let format = a["asset_url_format"].as_str()?;
            let url = |key: &str| a[key].as_str().filter(|f| !f.is_empty()).map(|f| format!("{CDN}{}", format.replace("${FILENAME}", f)));
            // `library_capsule` vaut 600×900 pour les jeux anciens (`library_600x900.jpg`),
            // 300×450 pour les récents (`<code>/library_capsule.jpg`) : leur 600×900 est alors le `_2x`.
            let portrait = match a["library_capsule"].as_str() {
                Some(f) if f.contains("600x900") => url("library_capsule"),
                _ => url("library_capsule_2x").or_else(|| url("library_capsule")),
            };
            Some((appid, Assets { portrait, header: url("header"), small_capsule: url("small_capsule") }))
        })
        .collect()
}

/// Jaquette verticale, avec repli sur l'image d'en-tête pour les jeux qui n'en ont pas.
/// Adresses exactes du magasin d'abord, puis les anciennes adresses fixes (si le magasin ne répond pas).
/// Renvoie (octets, type) où type vaut `portrait` ou `header`.
pub async fn download_cover(appid: u32, country: &str) -> Result<Option<(Vec<u8>, &'static str)>, String> {
    let known = assets(&[appid], country).await.ok().and_then(|mut m| m.remove(&appid)).unwrap_or_default();
    let candidates = [
        (known.portrait, "portrait"),
        (Some(cdn(appid, "library_600x900.jpg")), "portrait"),
        (known.header, "header"),
        (Some(cdn(appid, "header.jpg")), "header"),
    ];
    for (url, kind) in candidates {
        let Some(url) = url else { continue };
        if let Some(b) = get_bytes(&url).await? {
            return Ok(Some((b, kind)));
        }
    }
    Ok(None)
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_appid_et_liens() {
        let id = |s| parse_query(s);
        assert_eq!(id("1245620"), Query::AppId(1245620));
        assert_eq!(id(" https://store.steampowered.com/app/620/Portal_2/ "), Query::AppId(620));
        assert_eq!(id("steamcommunity.com/stats/1245620/achievements"), Query::AppId(1245620));
        assert_eq!(id("steam://rungameid/730"), Query::AppId(730));
        assert_eq!(id("steam://store/400"), Query::AppId(400));
        assert_eq!(id("Portal 2"), Query::Name("Portal 2".into()));
        assert_eq!(id("https://example.com/app/12"), Query::Name("https://example.com/app/12".into()));
    }

    #[test]
    fn lit_les_adresses_des_images() {
        let v = serde_json::json!({"response": {"store_items": [
            {"appid": 620, "success": 1, "assets": {"asset_url_format": "steam/apps/620/${FILENAME}?t=1",
                "library_capsule": "library_600x900.jpg", "library_capsule_2x": "library_600x900_2x.jpg",
                "header": "abc/header.jpg", "small_capsule": "abc/capsule_231x87.jpg"}},
            {"appid": 3764200, "success": 1, "assets": {"asset_url_format": "steam/apps/3764200/${FILENAME}?t=2",
                "library_capsule": "ed3b/library_capsule.jpg", "library_capsule_2x": "ed3b/library_capsule_2x.jpg"}},
            {"id": 999999999, "success": 15}
        ]}});
        let m = parse_assets(&v);
        assert_eq!(m.len(), 2);
        assert_eq!(m[&620].portrait.as_deref(), Some(&*format!("{CDN}steam/apps/620/library_600x900.jpg?t=1")));
        assert_eq!(m[&620].header.as_deref(), Some(&*format!("{CDN}steam/apps/620/abc/header.jpg?t=1")));
        assert_eq!(
            m[&3764200].portrait.as_deref(),
            Some(&*format!("{CDN}steam/apps/3764200/ed3b/library_capsule_2x.jpg?t=2"))
        );
        assert_eq!(m[&3764200].header, None);
    }

    #[test]
    fn encode_les_recherches() {
        assert_eq!(urlencode("Clair Obscur: été"), "Clair%20Obscur%3A%20%C3%A9t%C3%A9");
    }
}
