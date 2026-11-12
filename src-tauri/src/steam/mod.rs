//! Module Steam : tout ce qui lit des données Steam, isolé du reste de l'application
//! pour pouvoir être réparé seul si Valve change un format.
//!
//! Règle absolue : **lecture seule**. Rien ici n'écrit dans les fichiers de Steam
//! ni n'exécute de commande Steam.

pub mod detect;
pub mod kv_binary;
pub mod local_cache;
pub mod model;
pub mod web;

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct DetectReport {
    pub steam_path: Option<PathBuf>,
    pub accounts: Vec<detect::SteamAccount>,
    pub cached_games: Vec<local_cache::CachedGame>,
}

pub fn detect_all() -> DetectReport {
    let steam_path = detect::find_steam(None);
    let (accounts, cached_games) = match &steam_path {
        Some(p) => (detect::list_accounts(p), local_cache::list_cached_games(p)),
        None => (Vec::new(), Vec::new()),
    };
    DetectReport { steam_path, accounts, cached_games }
}

/// Résultat d'une source, sérialisé en `{ ok }` ou `{ err }`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome<T> {
    Ok(T),
    Err(String),
}

impl<T> From<Result<T, String>> for Outcome<T> {
    fn from(r: Result<T, String>) -> Self {
        match r {
            Ok(v) => Outcome::Ok(v),
            Err(e) => Outcome::Err(e),
        }
    }
}

/// Prototype de l'étape 2 : interroge chaque source séparément pour les comparer.
#[derive(Debug, Serialize)]
pub struct ProbeReport {
    pub appid: u32,
    pub local_cache: Outcome<model::GameAchievements>,
    pub public_profile: Outcome<model::GameAchievements>,
    pub public_page: Outcome<model::GameAchievements>,
    pub rarity_api_count: Outcome<usize>,
}

pub async fn probe(appid: u32, lang: &str, profile: Option<&str>) -> ProbeReport {
    let steam = detect::find_steam(None);
    let account = steam
        .as_deref()
        .and_then(|p| detect::list_accounts(p).into_iter().next());

    let rarity = web::load_rarity(appid).await;

    let mut local: Result<model::GameAchievements, String> = match &steam {
        Some(p) => local_cache::load(p, appid, account.as_ref().map(|a| a.account_id), lang),
        None => Err("Steam introuvable sur ce PC".into()),
    };
    // Le cache local n'a pas la rareté : on la complète depuis l'API publique.
    if let (Ok(g), Ok(r)) = (&mut local, &rarity) {
        for a in &mut g.achievements {
            a.rarity = a.api_name.as_ref().and_then(|n| r.get(n).copied());
        }
    }

    // Sans lien fourni, on essaie le profil du compte détecté (s'il est public).
    let profile_input = profile
        .map(str::to_string)
        .or_else(|| account.as_ref().map(|a| a.steamid64.to_string()));
    let public_profile = match &profile_input {
        Some(p) => web::load_profile(p, appid, lang).await,
        None => Err("aucun profil renseigné ni détecté".into()),
    };

    ProbeReport {
        appid,
        local_cache: local.into(),
        public_profile: public_profile.into(),
        public_page: web::load_public_page(appid, lang).await.into(),
        rarity_api_count: rarity.map(|r| r.len()).into(),
    }
}
