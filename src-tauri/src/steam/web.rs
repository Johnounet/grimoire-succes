//! Sources web publiques de Steam, sans clé API.
//!
//! Depuis 2026, les pages `steamcommunity.com/.../achievements/` sont rendues côté serveur
//! et embarquent leurs données dans `<script id="valve-ssr-data">` : un JSON dont
//! `renderContext.queryData` (lui-même du JSON en texte) contient des requêtes identifiées
//! par une clé, par ex. `["achievements", 620, "schema_v2"]`.
//! L'ancien `?xml=1` des profils ne renvoie plus de XML.

use super::model::{icon_url, Achievement, GameAchievements, Source};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(concat!("grimoire/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            .build()
            .expect("client HTTP")
    })
}

/// Télécharge un fichier binaire. `Ok(None)` si le serveur répond 404 (image absente).
pub(crate) async fn get_bytes(url: &str) -> Result<Option<Vec<u8>>, String> {
    let resp = client().get(url).send().await.map_err(|e| format!("requête vers {url} : {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(format!("{url} a répondu {}", resp.status()));
    }
    resp.bytes().await.map(|b| Some(b.to_vec())).map_err(|e| e.to_string())
}

pub(crate) async fn get_text(url: &str, lang: &str) -> Result<String, String> {
    let resp = client()
        .get(url)
        // La langue de la page suit ce cookie (le paramètre `?l=` est ignoré)
        .header(reqwest::header::COOKIE, format!("Steam_Language={lang}"))
        .send()
        .await
        .map_err(|e| format!("requête vers {url} : {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("{url} a répondu {status}"));
    }
    resp.text().await.map_err(|e| e.to_string())
}

/// Extrait les requêtes embarquées d'une page : (clé, données).
pub fn parse_ssr(html: &str) -> Result<Vec<(Value, Value)>, String> {
    const MARK: &str = "id=\"valve-ssr-data\"";
    let start = html.find(MARK).ok_or("bloc valve-ssr-data introuvable (format de page changé ?)")?;
    let start = start + html[start..].find('>').ok_or("balise script mal formée")? + 1;
    let end = start + html[start..].find("</script>").ok_or("balise script non fermée")?;

    let page: Value = serde_json::from_str(&html[start..end]).map_err(|e| format!("JSON de page : {e}"))?;
    let query_data = page
        .pointer("/renderContext/queryData")
        .and_then(Value::as_str)
        .ok_or("renderContext.queryData absent")?;
    let qd: Value = serde_json::from_str(query_data).map_err(|e| format!("queryData : {e}"))?;

    Ok(qd["queries"]
        .as_array()
        .map(|qs| {
            qs.iter()
                .map(|q| (q["queryKey"].clone(), q["state"]["data"].clone()))
                .collect()
        })
        .unwrap_or_default())
}

/// Données d'une requête `["achievements", <appid>, <kind>, ...]`.
fn find_query<'a>(queries: &'a [(Value, Value)], kind: &str) -> Option<&'a Value> {
    queries
        .iter()
        .find(|(k, d)| k[0] == "achievements" && k[2] == kind && !d.is_null())
        .map(|(_, d)| d)
}

fn as_f32(v: &Value) -> Option<f32> {
    v.as_f64().map(|f| f as f32).or_else(|| v.as_str()?.trim().parse().ok())
}

/// Construit la liste à partir de `schema_v2` (+ rareté et progression si présentes).
fn build(appid: u32, queries: &[(Value, Value)], source: Source) -> Result<GameAchievements, String> {
    let schema = find_query(queries, "schema_v2").ok_or("schéma des succès absent de la page")?;
    let percentages = find_query(queries, "globalpercentages").map(|p| &p["percentages"]);
    let user = find_query(queries, "user_achievements").map(|u| &u["achievements"]);

    let mut achievements = Vec::new();
    for group in schema["groups"].as_array().into_iter().flatten() {
        let dlc = group["dlcappid"].as_u64().filter(|&d| d != 0).map(|d| d as u32);
        for a in group["achievements"].as_array().into_iter().flatten() {
            let key = a["internal_key"].as_u64().map(|k| k.to_string()).unwrap_or_default();
            let user_entry = user.map(|u| &u[&key]);
            let s = |k: &str| a[k].as_str().unwrap_or_default().to_string();
            achievements.push(Achievement {
                api_name: a["api_name"].as_str().map(str::to_string).filter(|s| !s.is_empty()),
                name: s("name"),
                description: s("description"),
                hidden: a["hidden"].as_bool().unwrap_or(false),
                icon_url: icon_url(appid, &s("icon_achieved")),
                icon_gray_url: icon_url(appid, &s("icon_unachieved")),
                rarity: percentages.and_then(|p| as_f32(&p[&key])),
                unlocked: user_entry.map(|u| u["unlocked"].as_bool().unwrap_or(false)),
                unlock_time: user_entry.and_then(|u| u["unlock_time"].as_i64()).filter(|&t| t > 0),
                dlc_appid: dlc,
            });
        }
    }
    if achievements.is_empty() {
        return Err(format!("aucun succès trouvé pour le jeu {appid}"));
    }

    let game_name = queries
        .iter()
        .find(|(k, _)| k[0] == "StoreItem" && k[2] == "default_info")
        .and_then(|(_, d)| d["name"].as_str())
        .map(str::to_string);

    Ok(GameAchievements {
        appid,
        game_name,
        source,
        language: schema["language"].as_str().unwrap_or("english").to_string(),
        achievements,
    })
}

/// Page publique d'un jeu : tout sauf la progression.
pub async fn load_public_page(appid: u32, lang: &str) -> Result<GameAchievements, String> {
    let html = get_text(&format!("https://steamcommunity.com/stats/{appid}/achievements/"), lang).await?;
    build(appid, &parse_ssr(&html)?, Source::PublicPage)
}

/// Transforme ce que le joueur colle (lien de profil, SteamID64 ou pseudo d'URL)
/// en adresse de profil.
pub fn profile_base_url(input: &str) -> Result<String, String> {
    let s = input.trim().trim_end_matches('/');
    if s.len() == 17 && s.chars().all(|c| c.is_ascii_digit()) {
        return Ok(format!("https://steamcommunity.com/profiles/{s}"));
    }
    for (marker, kind) in [("/profiles/", "profiles"), ("/id/", "id")] {
        if let Some(i) = s.find(marker) {
            let id = s[i + marker.len()..].split(['/', '?', '#']).next().unwrap_or("");
            if !id.is_empty() {
                return Ok(format!("https://steamcommunity.com/{kind}/{id}"));
            }
        }
    }
    Err(format!("lien de profil Steam non reconnu : « {input} »"))
}

/// Profil public : schéma + progression du joueur.
pub async fn load_profile(profile: &str, appid: u32, lang: &str) -> Result<GameAchievements, String> {
    let url = format!("{}/stats/{appid}/achievements/", profile_base_url(profile)?);
    let queries = parse_ssr(&get_text(&url, lang).await?)?;
    if find_query(&queries, "user_achievements").is_none() {
        return Err("progression absente : profil ou détails de jeu privés, ou jeu non possédé".into());
    }
    build(appid, &queries, Source::PublicProfile)
}

/// Rareté mondiale par ID technique (API publique, sans clé).
pub async fn load_rarity(appid: u32) -> Result<HashMap<String, f32>, String> {
    let url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/?gameid={appid}"
    );
    let v: Value = serde_json::from_str(&get_text(&url, "english").await?).map_err(|e| e.to_string())?;
    let list = v
        .pointer("/achievementpercentages/achievements")
        .and_then(Value::as_array)
        .ok_or("réponse de rareté inattendue")?;
    Ok(list
        .iter()
        .filter_map(|a| Some((a["name"].as_str()?.to_string(), as_f32(&a["percent"])?)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_les_liens_de_profil() {
        let ok = |s| profile_base_url(s).unwrap();
        assert_eq!(ok("76561198071247993"), "https://steamcommunity.com/profiles/76561198071247993");
        assert_eq!(
            ok("https://steamcommunity.com/profiles/76561198071247993/"),
            "https://steamcommunity.com/profiles/76561198071247993"
        );
        assert_eq!(ok("steamcommunity.com/id/Johnounet/stats/620"), "https://steamcommunity.com/id/Johnounet");
        assert!(profile_base_url("bonjour").is_err());
    }

    #[test]
    fn lit_les_donnees_embarquees() {
        let qd = serde_json::json!({"queries": [
            {"queryKey": ["achievements", 620, "schema_v2"], "state": {"data": {"language": "french", "groups": [
                {"dlcappid": 0, "achievements": [{"internal_key": 1, "api_name": "ACH.A", "name": "A",
                  "description": "d", "hidden": true, "icon_achieved": "a.jpg", "icon_unachieved": "a_bw.jpg"}]}]}}},
            {"queryKey": ["achievements", 620, "globalpercentages"], "state": {"data": {"percentages": {"1": 12.5}}}},
            {"queryKey": ["achievements", 620, "user_achievements", "1"], "state": {"data": {"achievements":
                {"1": {"unlocked": true, "unlock_time": 1369056684}}}}}
        ]});
        let page = serde_json::json!({"renderContext": {"queryData": qd.to_string()}});
        let html = format!(r#"<script type="application/json" id="valve-ssr-data" nonce="x">{page}</script>"#);

        let g = build(620, &parse_ssr(&html).unwrap(), Source::PublicProfile).unwrap();
        let a = &g.achievements[0];
        assert_eq!(a.api_name.as_deref(), Some("ACH.A"));
        assert_eq!(a.rarity, Some(12.5));
        assert_eq!(a.unlocked, Some(true));
        assert_eq!(a.unlock_time, Some(1369056684));
        assert!(a.hidden);
        assert_eq!(g.language, "french");
    }
}
