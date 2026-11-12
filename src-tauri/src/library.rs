//! Bibliothèque de jeux : recherche, import, actualisation, suppression.
//! Fait le lien entre le module Steam (lecture) et la base SQLite (écriture).

use crate::db::{self, AchievementRow, GameSummary};
use crate::steam::{self, local_cache, store};
use rusqlite::Connection;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub images_dir: PathBuf,
}

impl AppState {
    pub(crate) fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        // Une écriture interrompue par un panic n'invalide pas la connexion SQLite
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn language(&self) -> String {
        db::get_setting(&self.conn(), "language").ok().flatten().unwrap_or_else(|| "french".into())
    }

    /// Pays du magasin Steam (code ISO), nécessaire à la recherche.
    fn country(&self) -> String {
        db::get_setting(&self.conn(), "country").ok().flatten().unwrap_or_else(|| "FR".into())
    }

    fn profile(&self) -> Option<String> {
        db::get_setting(&self.conn(), "profile_url").ok().flatten().filter(|s| !s.trim().is_empty())
    }
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub appid: u32,
    pub name: String,
    pub capsule_url: String,
    pub in_library: bool,
    pub on_this_pc: bool,
}

fn cached_games() -> Vec<local_cache::CachedGame> {
    steam::detect::find_steam(None).map(|p| local_cache::list_cached_games(&p)).unwrap_or_default()
}

fn hit(appid: u32, name: String, library: &[u32], cached: &[local_cache::CachedGame]) -> SearchHit {
    SearchHit {
        appid,
        name,
        capsule_url: store::capsule_url(appid),
        in_library: library.contains(&appid),
        on_this_pc: cached.iter().any(|c| c.appid == appid),
    }
}

#[tauri::command]
pub fn images_dir(state: State<'_, AppState>) -> PathBuf {
    state.images_dir.clone()
}

#[tauri::command]
pub fn library_list(state: State<'_, AppState>) -> Result<Vec<GameSummary>, String> {
    db::list_games(&state.conn()).map_err(err)
}

#[tauri::command]
pub fn game_achievements(state: State<'_, AppState>, appid: u32) -> Result<Vec<AchievementRow>, String> {
    db::list_achievements(&state.conn(), appid).map_err(err)
}

/// Jeux déjà lancés sur ce PC et pas encore importés (parcours « premier lancement »).
#[tauri::command]
pub fn library_local_suggestions(state: State<'_, AppState>) -> Result<Vec<SearchHit>, String> {
    let library = db::library_appids(&state.conn()).map_err(err)?;
    let cached = cached_games();
    let mut hits: Vec<SearchHit> = cached
        .iter()
        .filter(|c| !library.contains(&c.appid))
        .filter_map(|c| Some(hit(c.appid, c.name.clone()?, &library, &cached)))
        .collect();
    hits.sort_by_key(|h| h.name.to_lowercase());
    Ok(hits)
}

/// Recherche par nom, AppID ou lien Steam.
#[tauri::command]
pub async fn library_search(state: State<'_, AppState>, query: String) -> Result<Vec<SearchHit>, String> {
    let lang = state.language();
    let library = db::library_appids(&state.conn()).map_err(err)?;
    let cached = cached_games();

    match store::parse_query(&query) {
        store::Query::AppId(appid) => {
            let local_name = cached.iter().find(|c| c.appid == appid).and_then(|c| c.name.clone());
            let name = match store::app_name(appid, &lang).await {
                Ok(Some(n)) => Some(n),
                _ => local_name,
            };
            let name = name.ok_or_else(|| format!("Aucun jeu Steam ne porte l'AppID {appid}."))?;
            Ok(vec![hit(appid, name, &library, &cached)])
        }
        store::Query::Name(term) if term.chars().count() < 2 => Ok(Vec::new()),
        store::Query::Name(term) => {
            let needle = term.to_lowercase();
            // D'abord les jeux de ce PC qui correspondent, puis le magasin
            let mut hits: Vec<SearchHit> = cached
                .iter()
                .filter(|c| c.name.as_ref().is_some_and(|n| n.to_lowercase().contains(&needle)))
                .map(|c| hit(c.appid, c.name.clone().unwrap_or_default(), &library, &cached))
                .collect();
            hits.truncate(10);
            match store::search(&term, &lang, &state.country()).await {
                Ok(found) => {
                    for f in found {
                        if !hits.iter().any(|h| h.appid == f.appid) {
                            hits.push(hit(f.appid, f.name, &library, &cached));
                        }
                    }
                }
                Err(e) if hits.is_empty() => return Err(format!("Recherche impossible : {e}")),
                Err(_) => {}
            }
            Ok(hits)
        }
    }
}

/// Importe un jeu (ou le réimporte s'il existe déjà : c'est alors une actualisation).
#[tauri::command]
pub async fn library_import(state: State<'_, AppState>, appid: u32) -> Result<GameSummary, String> {
    let lang = state.language();
    let profile = state.profile();
    let data = steam::load_best(appid, &lang, profile.as_deref()).await?;

    let name = match &data.game_name {
        Some(n) => n.clone(),
        None => store::app_name(appid, &lang).await.ok().flatten().unwrap_or_else(|| format!("AppID {appid}")),
    };

    // Jaquette : téléchargée une seule fois, pour fonctionner hors ligne
    let already = db::get_game(&state.conn(), appid).map_err(err)?;
    let mut cover: Option<(String, &str)> = None;
    if already.as_ref().and_then(|g| g.cover_file.as_ref()).is_none() {
        if let Ok(Some((bytes, kind))) = store::download_cover(appid).await {
            let rel = format!("covers/{appid}.jpg");
            let path = state.images_dir.join(&rel);
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(err)?;
            }
            std::fs::write(&path, bytes).map_err(err)?;
            cover = Some((rel, kind));
        }
    }

    let mut conn = state.conn();
    db::save_game(&mut conn, &data, &name, cover.as_ref().map(|(f, k)| (f.as_str(), *k))).map_err(err)?;
    db::get_game(&conn, appid).map_err(err)?.ok_or_else(|| "jeu introuvable après import".into())
}

/// Coche ou décoche un succès à la main. Renvoie le jeu avec sa progression à jour.
#[tauri::command]
pub fn achievement_set_unlocked(state: State<'_, AppState>, id: i64, unlocked: bool) -> Result<GameSummary, String> {
    let conn = state.conn();
    let appid = db::set_unlocked(&conn, id, unlocked).map_err(err)?;
    db::get_game(&conn, appid).map_err(err)?.ok_or_else(|| "jeu introuvable".into())
}

#[tauri::command]
pub fn workspace_list(state: State<'_, AppState>, appid: u32) -> Result<Vec<db::WorkspaceItem>, String> {
    db::list_workspace(&state.conn(), appid).map_err(err)
}

#[tauri::command]
pub fn workspace_save(state: State<'_, AppState>, appid: u32, items: Vec<db::WorkspaceItem>) -> Result<(), String> {
    db::save_workspace(&mut state.conn(), appid, &items).map_err(err)
}

/// Réglages modifiables depuis l'interface.
const SETTINGS: &[&str] = &["anti_spoiler", "language", "country", "profile_url", "workspace_view", "panel_open"];

fn check_setting(key: &str) -> Result<(), String> {
    if SETTINGS.contains(&key) {
        Ok(())
    } else {
        Err(format!("réglage inconnu : {key}"))
    }
}

#[tauri::command]
pub fn setting_get(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    check_setting(&key)?;
    db::get_setting(&state.conn(), &key).map_err(err)
}

#[tauri::command]
pub fn setting_set(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    check_setting(&key)?;
    db::set_setting(&state.conn(), &key, &value).map_err(err)
}

#[tauri::command]
pub fn library_delete(state: State<'_, AppState>, appid: u32) -> Result<(), String> {
    if !db::game_exists(&state.conn(), appid).map_err(err)? {
        return Ok(());
    }
    let cover = db::delete_game(&state.conn(), appid).map_err(err)?;
    if let Some(rel) = cover {
        let _ = std::fs::remove_file(state.images_dir.join(rel));
    }
    Ok(())
}
