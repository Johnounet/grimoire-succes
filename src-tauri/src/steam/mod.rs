//! Module Steam : tout ce qui lit des données Steam, isolé du reste de l'application
//! pour pouvoir être réparé seul si Valve change un format.
//!
//! Règle absolue : **lecture seule**. Rien ici n'écrit dans les fichiers de Steam
//! ni n'exécute de commande Steam.

pub mod detect;
pub mod kv_binary;
pub mod local_cache;
pub mod manifests;
pub mod model;
pub mod playtime;
pub mod store;
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

/// Import d'un jeu depuis la meilleure source disponible (ordre du SPEC) :
/// 1. cache local, 2. profil public, 3. (clé API : plus tard), 4. page publique.
///
/// `profile` : lien de profil réglé par le joueur ; à défaut, le compte détecté sur le PC.
/// Dossier de Steam et compte de ce PC (le premier trouvé, comme pour l'import).
pub fn local_account() -> Option<(PathBuf, u32)> {
    let steam = detect::find_steam(None)?;
    let account = detect::list_accounts(&steam).into_iter().next()?;
    Some((steam, account.account_id))
}

pub async fn load_best(appid: u32, lang: &str, profile: Option<&str>) -> Result<model::GameAchievements, String> {
    let steam = detect::find_steam(None);
    let account = steam.as_deref().and_then(|p| detect::list_accounts(p).into_iter().next());
    let profile = profile
        .map(str::to_string)
        .or_else(|| account.as_ref().map(|a| a.steamid64.to_string()));

    let mut errors = Vec::new();

    if let Some(p) = &steam {
        match local_cache::load(p, appid, account.as_ref().map(|a| a.account_id), lang) {
            Ok(mut g) => {
                // Le cache local n'a pas la rareté ; sans fichier de progression, il n'a pas non plus l'état.
                if let Ok(r) = web::load_rarity(appid).await {
                    for a in &mut g.achievements {
                        a.rarity = a.api_name.as_ref().and_then(|n| r.get(n).copied());
                    }
                }
                if g.achievements.iter().all(|a| a.unlocked.is_none()) {
                    if let Some(prof) = &profile {
                        if let Ok(pg) = web::load_profile(prof, appid, lang).await {
                            merge_progress(&mut g, &pg);
                        }
                    }
                }
                return Ok(g);
            }
            Err(e) => errors.push(e),
        }
    }

    if let Some(prof) = &profile {
        match web::load_profile(prof, appid, lang).await {
            Ok(g) => return Ok(g),
            Err(e) => errors.push(format!("profil : {e}")),
        }
    }

    web::load_public_page(appid, lang).await.map_err(|e| {
        errors.push(format!("page publique : {e}"));
        errors.join(" ; ")
    })
}

/// Temps de jeu Steam (minutes) du compte détecté sur ce PC, s'il est connu.
pub fn playtime_minutes(appid: u32) -> Option<u32> {
    let steam = detect::find_steam(None)?;
    let account = detect::list_accounts(&steam).into_iter().next()?;
    playtime::load(&steam, account.account_id, appid)
}

fn merge_progress(target: &mut model::GameAchievements, from: &model::GameAchievements) {
    for a in &mut target.achievements {
        if let Some(src) = from.achievements.iter().find(|s| s.api_name.is_some() && s.api_name == a.api_name) {
            a.unlocked = src.unlocked;
            a.unlock_time = src.unlock_time;
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
