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
/// Lectures ratées de suite pour une même modification avant d'abandonner (jusqu'à la suivante) :
/// laisse à Steam le temps de finir d'écrire, sans relire sans fin un fichier abîmé.
const MAX_FAILURES: u32 = 5;

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
    // Lectures ratées de suite depuis la dernière modification, par jeu
    let mut failures: HashMap<u32, u32> = HashMap::new();

    loop {
        std::thread::sleep(POLL);
        // Compte relu de temps en temps, même trouvé : Steam a pu changer de compte connecté
        if last_detect.is_none_or(|t| t.elapsed() >= REDETECT) {
            last_detect = Some(Instant::now());
            if switch_account(&mut steam, steam::local_account()) {
                // Les fichiers du nouveau compte servent de nouvelle référence, pas de « changements »
                seen.clear();
                states.clear();
                failures.clear();
            }
        }
        let Some((path, account)) = &steam else { continue };
        let state = app.state::<AppState>();
        let appids = db::library_appids(&state.conn()).unwrap_or_default();

        for appid in appids {
            let file = local_cache::progress_path(path, *account, appid);
            let Ok(mtime) = std::fs::metadata(&file).and_then(|m| m.modified()) else { continue };
            if !file_changed(&mut seen, appid, mtime) {
                continue;
            }
            // Lecture ratée (fichier verrouillé ou à moitié écrit par Steam) : la nouvelle date n'est
            // pas retenue, le passage suivant réessaie, MAX_FAILURES fois au plus
            let lang = state.language();
            let read = local_cache::load(path, appid, Some(*account), &lang);
            let Some(data) = read.as_ref().ok().filter(|d| state_known(d)) else {
                if give_up(&mut failures, appid) {
                    let why = read.err().unwrap_or_else(|| "état des succès illisible".into());
                    eprintln!("actualisation automatique de {appid} abandonnée jusqu'à la prochaine modification : {why}");
                    seen.insert(appid, mtime);
                }
                continue;
            };
            failures.remove(&appid);
            seen.insert(appid, mtime);
            let now = signature(data);
            if states.insert(appid, now.clone()).as_ref() == Some(&now) {
                continue;
            }
            match refresh(&state, appid, data) {
                Ok(Some(event)) => {
                    let _ = app.emit("steam-progress", event);
                }
                Ok(None) => {}
                Err(e) => eprintln!("actualisation automatique de {appid} : {e}"),
            }
        }
    }
}

/// Garde le compte détecté. Steam momentanément introuvable : on garde le compte d'avant.
/// Renvoie `true` si on passe d'un compte (ou dossier Steam) à un autre.
fn switch_account(current: &mut Option<(PathBuf, u32)>, found: Option<(PathBuf, u32)>) -> bool {
    let Some(found) = found else { return false };
    let changed = current.as_ref().is_some_and(|c| *c != found);
    *current = Some(found);
    changed
}

/// Le fichier a-t-il changé depuis la dernière lecture réussie ? La première date vue sert de
/// référence (retenue tout de suite, sans lecture).
fn file_changed(seen: &mut HashMap<u32, SystemTime>, appid: u32, mtime: SystemTime) -> bool {
    match seen.get(&appid) {
        None => {
            seen.insert(appid, mtime);
            false
        }
        Some(prev) => *prev != mtime,
    }
}

/// Compte une lecture ratée. `true` au bout de `MAX_FAILURES` de suite : abandonner (compteur remis à zéro).
fn give_up(failures: &mut HashMap<u32, u32>, appid: u32) -> bool {
    let n = failures.entry(appid).or_insert(0);
    *n += 1;
    if *n < MAX_FAILURES {
        return false;
    }
    failures.remove(&appid);
    true
}

/// Lecture utilisable : l'état des succès est connu. Un fichier de progression illisible donne un
/// état « inconnu », qui ne change rien en base : à relire plus tard.
fn state_known(data: &GameAchievements) -> bool {
    data.achievements.iter().all(|a| a.unlocked.is_some())
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
    fn suit_le_changement_de_compte() {
        let a = || Some((PathBuf::from("steam"), 1));
        let b = || Some((PathBuf::from("steam"), 2));
        let mut cur = None;
        assert!(!switch_account(&mut cur, None)); // Steam introuvable
        assert!(!switch_account(&mut cur, a())); // premier compte trouvé : pas un changement
        assert!(!switch_account(&mut cur, a())); // même compte
        assert!(!switch_account(&mut cur, None)); // introuvable un moment : compte gardé
        assert_eq!(cur, a());
        assert!(switch_account(&mut cur, b())); // passage au compte B
        assert_eq!(cur, b());
    }

    #[test]
    fn date_retenue_seulement_apres_lecture_reussie() {
        let mut seen = HashMap::new();
        let t0 = SystemTime::UNIX_EPOCH;
        let t1 = t0 + Duration::from_secs(1);
        assert!(!file_changed(&mut seen, 620, t0)); // première vue : référence
        assert!(!file_changed(&mut seen, 620, t0));
        assert!(file_changed(&mut seen, 620, t1));
        // Lecture ratée : date non retenue, le passage suivant réessaie
        assert!(file_changed(&mut seen, 620, t1));
        seen.insert(620, t1); // lecture réussie
        assert!(!file_changed(&mut seen, 620, t1));

        let mut failures = HashMap::new();
        for _ in 1..MAX_FAILURES {
            assert!(!give_up(&mut failures, 620));
        }
        assert!(give_up(&mut failures, 620)); // abandon, compteur remis à zéro
        assert!(!give_up(&mut failures, 620));

        assert!(state_known(&game(vec![ach("A", Some(false)), ach("B", Some(true))])));
        assert!(!state_known(&game(vec![ach("A", None), ach("B", None)])));
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
