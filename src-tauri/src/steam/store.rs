//! Magasin Steam : recherche de jeux, nom officiel, jaquettes.

use super::web::{get_bytes, get_text};
use serde::Serialize;
use serde_json::Value;

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

fn cdn(appid: u32, file: &str) -> String {
    format!("https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/{appid}/{file}")
}

/// Petite image horizontale, pour les listes de résultats.
pub fn capsule_url(appid: u32) -> String {
    cdn(appid, "capsule_231x87.jpg")
}

pub fn hero_url(appid: u32) -> String {
    cdn(appid, "library_hero.jpg")
}

/// Jaquette verticale, avec repli sur l'image d'en-tête pour les vieux jeux.
/// Renvoie (octets, type) où type vaut `portrait` ou `header`.
pub async fn download_cover(appid: u32) -> Result<Option<(Vec<u8>, &'static str)>, String> {
    if let Some(b) = get_bytes(&cdn(appid, "library_600x900.jpg")).await? {
        return Ok(Some((b, "portrait")));
    }
    Ok(get_bytes(&cdn(appid, "header.jpg")).await?.map(|b| (b, "header")))
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
    fn encode_les_recherches() {
        assert_eq!(urlencode("Clair Obscur: été"), "Clair%20Obscur%3A%20%C3%A9t%C3%A9");
    }
}
