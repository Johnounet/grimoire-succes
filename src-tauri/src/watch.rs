//! Actualisation automatique : Steam réécrit le fichier de progression d'un jeu
//! (`appcache/stats/UserGameStats_<compte>_<appid>.bin`) dès qu'un succès est débloqué, sans attendre
//! la fermeture du jeu. On surveille la date de modification de ces fichiers pour les jeux de la
//! bibliothèque, et un changement relit le seul cache local (sans réseau : la rareté déjà en base est
//! gardée). Les règles d'actualisation de `db::save_game` s'appliquent (coches manuelles comprises).
//!
//! Le même fichier porte les statistiques du jeu : une progression suivie par Steam (« 37 / 50 ») qui
//! change est enregistrée aussi, et annoncée par le même événement (sans succès débloqué).
//!
//! Lecture seule côté Steam : on ne lit que des dates de fichiers et le cache, jamais d'écriture.

use crate::db::{self, GameSummary};
use crate::library::AppState;
use crate::steam::{self, local_cache, model::GameAchievements};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};
use tauri::{AppHandle, Emitter, Manager};

/// Délai entre deux coups d'œil aux fichiers : assez court pour un retour « en direct ».
const POLL: Duration = Duration::from_secs(2);
/// Steam introuvable (pas installé, pas encore connecté) : nouvel essai de temps en temps.
const REDETECT: Duration = Duration::from_secs(60);

/// Événement `steam-progress`, reçu par les deux fenêtres.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub game: GameSummary,
    /// Succès qui viennent de passer à « obtenu »
    pub unlocked: Vec<Unlocked>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Unlocked {
    pub id: i64,
    pub name: String,
}

pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("steam-watch".into())
        .spawn(move || run(app))
        .ok();
}

fn run(app: AppHandle) {
    let mut steam: Option<(PathBuf, u32)> = None;
    let mut last_detect: Option<Instant> = None;
    // Date de modification vue en dernier, par jeu (la première vue sert de référence)
    let mut seen: HashMap<u32, SystemTime> = HashMap::new();
    // États lus en dernier dans chaque fichier : un fichier réécrit sans changement d'état ni de
    // progression des succès (autres statistiques du jeu) ne touche pas à la base
    let mut states: HashMap<u32, Vec<Signature>> = HashMap::new();

    loop {
        std::thread::sleep(POLL);
        if steam.is_none() && last_detect.is_none_or(|t| t.elapsed() >= REDETECT) {
            last_detect = Some(Instant::now());
            steam = steam::local_account();
        }
        let Some((path, account)) = &steam else { continue };
        let state = app.state::<AppState>();
        let appids = db::library_appids(&state.conn()).unwrap_or_default();

        for appid in appids {
            let file = local_cache::progress_path(path, *account, appid);
            let Ok(mtime) = std::fs::metadata(&file).and_then(|m| m.modified()) else { continue };
            match seen.insert(appid, mtime) {
                Some(prev) if prev != mtime => {}
                _ => continue,
            }
            let lang = state.language();
            let Ok(data) = local_cache::load(path, appid, Some(*account), &lang) else { continue };
            let now = signature(&data);
            if states.insert(appid, now.clone()).as_ref() == Some(&now) {
                continue;
            }
            match refresh(&state, appid, &data) {
                Ok(Some(event)) => {
                    let _ = app.emit("steam-progress", event);
                }
                Ok(None) => {}
                Err(e) => eprintln!("actualisation automatique de {appid} : {e}"),
            }
        }
    }
}

/// État et progression d'un succès dans le fichier (clé : ID technique, sinon nom).
type Signature = (String, Option<bool>, Option<f64>);

fn signature(data: &GameAchievements) -> Vec<Signature> {
    data.achievements
        .iter()
        .map(|a| {
            let key = a.api_name.clone().unwrap_or_else(|| format!("name:{}", a.name));
            (key, a.unlocked, a.progress.map(|p| p.value))
        })
        .collect()
}

/// Enregistre la progression lue. Renvoie l'événement à envoyer si l'état ou la progression des succès
/// a changé en base.
fn refresh(state: &AppState, appid: u32, data: &GameAchievements) -> rusqlite::Result<Option<ProgressEvent>> {
    let mut conn = state.conn();
    let Some(game) = db::get_game(&conn, appid)? else { return Ok(None) };
    let before = db::list_achievements(&conn, appid)?;
    db::save_game(&mut conn, data, &game.name, None)?;
    let after = db::list_achievements(&conn, appid)?;

    let unlocked = newly_unlocked(&before, &after);
    let state = |list: &[db::AchievementRow]| list.iter().map(|a| (a.id, a.unlocked, a.progress)).collect::<Vec<_>>();
    let changed = state(&before) != state(&after);
    if !changed {
        return Ok(None);
    }
    let Some(game) = db::get_game(&conn, appid)? else { return Ok(None) };
    Ok(Some(ProgressEvent { game, unlocked }))
}

/// Succès non retirés qui sont obtenus maintenant et ne l'étaient pas avant.
fn newly_unlocked(before: &[db::AchievementRow], after: &[db::AchievementRow]) -> Vec<Unlocked> {
    let had: HashSet<i64> = before.iter().filter(|a| a.unlocked).map(|a| a.id).collect();
    after
        .iter()
        .filter(|a| a.unlocked && !a.removed && !had.contains(&a.id))
        .map(|a| Unlocked { id: a.id, name: a.name.clone() })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::steam::model::{Achievement, Progress, Source};

    fn ach(api: &str, unlocked: Option<bool>) -> Achievement {
        Achievement {
            api_name: Some(api.into()),
            name: api.to_lowercase(),
            description: String::new(),
            hidden: false,
            icon_url: None,
            icon_gray_url: None,
            rarity: None,
            unlocked,
            unlock_time: None,
            dlc_appid: None,
            progress: None,
        }
    }

    fn game(achievements: Vec<Achievement>) -> GameAchievements {
        GameAchievements {
            appid: 620,
            game_name: None,
            source: Source::LocalCache,
            language: "french".into(),
            achievements,
        }
    }

    #[test]
    fn detecte_les_succes_debloques() {
        let state = AppState {
            db: std::sync::Mutex::new(db::open_in_memory().unwrap()),
            data_dir: PathBuf::new(),
            images_dir: PathBuf::new(),
        };
        let mut first = game(vec![ach("A", Some(false)), ach("B", Some(false))]);
        first.achievements[0].rarity = Some(4.2);
        db::save_game(&mut state.conn(), &first, "Portal 2", None).unwrap();

        // Rien n'a changé : pas d'événement
        assert!(refresh(&state, 620, &game(vec![ach("A", Some(false)), ach("B", Some(false))])).unwrap().is_none());

        // B débloqué en jeu : événement, nom du jeu et rareté conservés (le cache local n'en a pas)
        let ev = refresh(&state, 620, &game(vec![ach("A", Some(false)), ach("B", Some(true))])).unwrap().unwrap();
        assert_eq!(ev.unlocked, vec![Unlocked { id: ev.unlocked[0].id, name: "b".into() }]);
        assert_eq!((ev.game.name.as_str(), ev.game.unlocked), ("Portal 2", 1));
        let list = db::list_achievements(&state.conn(), 620).unwrap();
        assert_eq!(list[0].rarity, Some(4.2));

        // Jeu absent de la bibliothèque : ignoré
        assert!(refresh(&state, 999, &game(vec![ach("A", Some(true))])).unwrap().is_none());
        assert_eq!(signature(&game(vec![ach("A", None)])), vec![("A".to_string(), None, None)]);

        // Progression qui avance sans succès débloqué : événement sans succès
        let with = |v: f64| {
            let mut g = game(vec![ach("A", Some(false)), ach("B", Some(true))]);
            g.achievements[0].progress = Some(Progress { value: v, min: 0.0, max: 50.0 });
            g
        };
        let ev = refresh(&state, 620, &with(37.0)).unwrap().unwrap();
        assert!(ev.unlocked.is_empty());
        assert_eq!(db::list_achievements(&state.conn(), 620).unwrap()[0].progress.map(|p| p.value), Some(37.0));
        assert!(refresh(&state, 620, &with(37.0)).unwrap().is_none());
        // Source sans progression (profil public) : la dernière valeur connue est gardée
        refresh(&state, 620, &game(vec![ach("A", Some(false)), ach("B", Some(true))])).unwrap();
        assert_eq!(db::list_achievements(&state.conn(), 620).unwrap()[0].progress.map(|p| p.value), Some(37.0));
    }
}
