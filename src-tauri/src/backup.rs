//! Sauvegardes de sécurité des données du joueur (guides, coches, étiquettes, réglages).
//!
//! Une sauvegarde est un dossier `backups/<horodatage>-<sorte>/` dans le dossier des données (nom
//! neutre, qui ne changera pas avec la langue ; l'ancien `sauvegardes/` est renommé au démarrage) :
//! - `donnees.sqlite` : copie cohérente de la base (API de sauvegarde de SQLite, base ouverte) ;
//! - `images/blocks/` : images des blocs utilisées par cette base, en liens physiques (elles ne sont
//!   jamais réécrites : un nom par image), copiées si le lien est impossible ;
//! - `images/covers/` : jaquettes, copiées (petites, et réécrites si un jeu est rajouté) ;
//! - `info.json` : date, sorte, nombre de jeux.
//!
//! Sortes : `auto` (une par jour, au démarrage puis toutes les heures si l'application reste ouverte ;
//! les 7 dernières sont gardées), `manuel` (bouton des Réglages) et `avant-restauration` (état remplacé
//! par une restauration, pour pouvoir revenir en arrière) et `avant-mise-a-jour` (base telle
//! qu'elle était avant les migrations d'une nouvelle version du logiciel ; les 3 dernières gardées).
//! Les manuelles et `avant-restauration` ne sont jamais supprimées d'office.

use crate::db;
use crate::library::AppState;
use rusqlite::{backup::Progress, Connection, OpenFlags, MAIN_DB};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};

/// Sauvegardes automatiques gardées.
pub const KEEP_AUTO: usize = 7;
/// Sauvegardes d'avant mise à jour gardées.
pub const KEEP_BEFORE_UPDATE: usize = 3;
/// Écart entre deux sauvegardes automatiques.
const DAY: i64 = 24 * 3600;
/// Coup d'œil pendant que l'application reste ouverte.
const CHECK_EVERY: Duration = Duration::from_secs(3600);

use crate::data_dir::DB_FILE;
const INFO_FILE: &str = "info.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Auto,
    Manuel,
    AvantRestauration,
    AvantMiseAJour,
}

impl Kind {
    fn slug(self) -> &'static str {
        match self {
            Kind::Auto => "auto",
            Kind::Manuel => "manuel",
            Kind::AvantRestauration => "avant-restauration",
            Kind::AvantMiseAJour => "avant-mise-a-jour",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    /// Nom du dossier, identifiant de la sauvegarde.
    #[serde(default)]
    pub id: String,
    pub created_at: i64,
    pub kind: Kind,
    pub games: u32,
    /// Taille en octets (base et images copiées).
    #[serde(default)]
    pub size: u64,
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

/// Premières versions : le dossier s'appelait `sauvegardes`.
fn rename_old_dir(data_dir: &Path) {
    let old = data_dir.join("sauvegardes");
    if old.is_dir() && !backups_dir(data_dir).exists() {
        let _ = std::fs::rename(old, backups_dir(data_dir));
    }
}

/// Dossier d'une sauvegarde d'après son identifiant, refusé s'il sort du dossier des sauvegardes.
fn backup_path(data_dir: &Path, id: &str) -> Result<PathBuf, String> {
    let ok = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !ok {
        return Err("sauvegarde inconnue".into());
    }
    let path = backups_dir(data_dir).join(id);
    if !path.join(INFO_FILE).is_file() {
        return Err("sauvegarde introuvable".into());
    }
    Ok(path)
}

/// Copie (ou lien physique) d'un fichier, dossiers créés au besoin.
fn put(src: &Path, dst: &Path, link: bool) -> std::io::Result<u64> {
    if let Some(dir) = dst.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if link && std::fs::hard_link(src, dst).is_ok() {
        return Ok(0);
    }
    std::fs::copy(src, dst)
}

/// Images d'une base : jaquettes des jeux et images des blocs (chemins relatifs à `images/`).
fn referenced_images(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut out: Vec<String> = db::referenced_block_images(conn)?.into_iter().collect();
    let mut stmt = conn.prepare("SELECT cover_file FROM games WHERE cover_file IS NOT NULL")?;
    out.extend(stmt.query_map([], |r| r.get::<_, String>(0))?.flatten());
    Ok(out)
}

/// Chemin relatif sans remontée de dossier (on ne copie jamais hors de `images/`).
fn safe_rel(rel: &str) -> bool {
    !rel.is_empty() && !rel.contains("..") && !rel.starts_with(['/', '\\']) && !rel.contains(':')
}

/// Crée une sauvegarde de la base ouverte et de ses images.
pub fn create(conn: &Connection, data_dir: &Path, images_dir: &Path, kind: Kind) -> Result<BackupInfo, String> {
    let created_at = now();
    let root = backups_dir(data_dir);
    // Deux sauvegardes dans la même seconde : suffixe
    let mut id = format!("{created_at}-{}", kind.slug());
    let mut n = 1;
    while root.join(&id).exists() {
        n += 1;
        id = format!("{created_at}-{}-{n}", kind.slug());
    }
    let dir = root.join(&id);
    let tmp = root.join(format!(".{id}.tmp"));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(err)?;

    let result = (|| -> Result<BackupInfo, String> {
        conn.backup(MAIN_DB, tmp.join(DB_FILE), None::<fn(Progress)>).map_err(err)?;
        let mut size = std::fs::metadata(tmp.join(DB_FILE)).map(|m| m.len()).unwrap_or(0);
        // Avant une migration, la base suit un ancien schéma : les requêtes d'aujourd'hui peuvent
        // ne pas s'y appliquer. La copie de la base est l'essentiel (les migrations ne touchent pas
        // aux images), le reste est fait si possible.
        let lenient = kind == Kind::AvantMiseAJour;
        let images = match referenced_images(conn) {
            Err(_) if lenient => Vec::new(),
            r => r.map_err(err)?,
        };
        for rel in images {
            let src = images_dir.join(&rel);
            if safe_rel(&rel) && src.is_file() {
                size += put(&src, &tmp.join("images").join(&rel), rel.starts_with("blocks/")).map_err(err)?;
            }
        }
        let games = match db::library_appids(conn) {
            Err(_) if lenient => 0,
            r => r.map_err(err)?.len() as u32,
        };
        let info = BackupInfo { id: id.clone(), created_at, kind, games, size };
        std::fs::write(tmp.join(INFO_FILE), serde_json::to_vec_pretty(&info).map_err(err)?).map_err(err)?;
        Ok(info)
    })();
    match result {
        Ok(info) => {
            // Renommée seulement une fois complète : une sauvegarde listée est toujours entière
            std::fs::rename(&tmp, &dir).map_err(err)?;
            Ok(info)
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            Err(e)
        }
    }
}

/// Sauvegardes existantes, la plus récente d'abord.
pub fn list(data_dir: &Path) -> Vec<BackupInfo> {
    let Ok(entries) = std::fs::read_dir(backups_dir(data_dir)) else { return Vec::new() };
    let mut out: Vec<BackupInfo> = entries
        .flatten()
        .filter_map(|e| {
            let id = e.file_name().to_str()?.to_string();
            if id.starts_with('.') {
                return None;
            }
            let mut info: BackupInfo = serde_json::from_slice(&std::fs::read(e.path().join(INFO_FILE)).ok()?).ok()?;
            info.id = id;
            Some(info)
        })
        .collect();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
    out
}

/// Garde les `keep` sauvegardes de la sorte `kind` les plus récentes.
fn prune(data_dir: &Path, kind: Kind, keep: usize) {
    for old in list(data_dir).into_iter().filter(|b| b.kind == kind).skip(keep) {
        let _ = std::fs::remove_dir_all(backups_dir(data_dir).join(&old.id));
    }
}

/// Au démarrage, avant les migrations : copie de la base si une nouvelle version du logiciel va
/// la modifier. Une migration défectueuse laisse ainsi une copie saine du jour même.
pub fn before_migration(conn: &Connection, data_dir: &Path, images_dir: &Path) -> Result<Option<BackupInfo>, String> {
    if !db::needs_migration(conn).map_err(err)? {
        return Ok(None);
    }
    // Sinon le dossier `backups` créé ici empêcherait de reprendre l'ancien `sauvegardes`
    rename_old_dir(data_dir);
    let info = create(conn, data_dir, images_dir, Kind::AvantMiseAJour)?;
    prune(data_dir, Kind::AvantMiseAJour, KEEP_BEFORE_UPDATE);
    Ok(Some(info))
}

/// Sauvegarde automatique si la dernière date d'un jour ou plus (ou s'il n'y en a pas).
pub fn auto_if_due(conn: &Connection, data_dir: &Path, images_dir: &Path) -> Result<Option<BackupInfo>, String> {
    let last = list(data_dir).into_iter().find(|b| b.kind == Kind::Auto).map(|b| b.created_at);
    if last.is_some_and(|t| now() - t < DAY) {
        return Ok(None);
    }
    // Rien à protéger tant que la bibliothèque est vide (premier lancement)
    if db::library_appids(conn).map_err(err)?.is_empty() {
        return Ok(None);
    }
    let info = create(conn, data_dir, images_dir, Kind::Auto)?;
    prune(data_dir, Kind::Auto, KEEP_AUTO);
    Ok(Some(info))
}

/// Base d'une sauvegarde utilisable : lisible, pas d'une version plus récente du logiciel (on ne
/// saurait pas la lire), et saine (`quick_check` : une sauvegarde à moitié abîmée ne doit pas
/// remplacer des données).
fn check_usable(src: &Path) -> Result<(), String> {
    let unreadable = |e: rusqlite::Error| format!("sauvegarde illisible : {e}");
    let conn = Connection::open_with_flags(src, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(unreadable)?;
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).map_err(unreadable)?;
    if version as usize > db::schema_version() {
        return Err("cette sauvegarde vient d'une version plus récente de Grimoire".into());
    }
    match db::quick_check(&conn).map_err(unreadable)? {
        Some(problem) => Err(format!("sauvegarde abîmée : {}", problem.lines().next().unwrap_or_default())),
        None => Ok(()),
    }
}

/// Remet les données d'une sauvegarde à la place des données actuelles, après avoir sauvegardé
/// celles-ci (sorte `avant-restauration`).
pub fn restore(conn: &mut Connection, data_dir: &Path, images_dir: &Path, id: &str) -> Result<(), String> {
    let dir = backup_path(data_dir, id)?;
    let src = dir.join(DB_FILE);
    check_usable(&src)?;

    create(conn, data_dir, images_dir, Kind::AvantRestauration)?;
    conn.restore(MAIN_DB, &src, None::<fn(Progress)>).map_err(err)?;
    db::migrate_existing(conn).map_err(err)?;
    copy_missing_images(conn, &dir, images_dir)
}

/// Remet les images d'une sauvegarde absentes des données actuelles.
fn copy_missing_images(conn: &Connection, dir: &Path, images_dir: &Path) -> Result<(), String> {
    for rel in referenced_images(conn).map_err(err)? {
        let from = dir.join("images").join(&rel);
        let to = images_dir.join(&rel);
        if safe_rel(&rel) && from.is_file() && !to.exists() {
            put(&from, &to, rel.starts_with("blocks/")).map_err(err)?;
        }
    }
    Ok(())
}

/// Base abîmée, déjà mise de côté (`recovery.rs`) : remet la plus récente des sauvegardes
/// utilisables à sa place et l'ouvre. Une sauvegarde illisible, abîmée ou d'une version plus
/// récente du logiciel est passée au profit de la suivante.
pub fn restore_latest_offline(data_dir: &Path, images_dir: &Path) -> Result<Connection, String> {
    let target = data_dir.join(DB_FILE);
    let mut last_error = "aucune sauvegarde".to_string();
    for info in list(data_dir) {
        let dir = backups_dir(data_dir).join(&info.id);
        let attempt = (|| -> Result<Connection, String> {
            check_usable(&dir.join(DB_FILE))?;
            std::fs::copy(dir.join(DB_FILE), &target).map_err(err)?;
            let conn = db::open(&target).map_err(err)?;
            copy_missing_images(&conn, &dir, images_dir)?;
            Ok(conn)
        })();
        match attempt {
            Ok(conn) => return Ok(conn),
            Err(e) => {
                last_error = e;
                for suffix in ["", "-wal", "-shm"] {
                    let _ = std::fs::remove_file(data_dir.join(format!("{DB_FILE}{suffix}")));
                }
            }
        }
    }
    Err(last_error)
}

pub fn delete(data_dir: &Path, id: &str) -> Result<(), String> {
    std::fs::remove_dir_all(backup_path(data_dir, id)?).map_err(err)
}

/// Au démarrage, puis toutes les heures : sauvegarde automatique si elle est due.
pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("sauvegardes".into())
        .spawn(move || loop {
            {
                let state = app.state::<AppState>();
                rename_old_dir(&state.data_dir);
                let conn = state.conn();
                if let Err(e) = auto_if_due(&conn, &state.data_dir, &state.images_dir) {
                    eprintln!("sauvegarde automatique : {e}");
                }
            }
            std::thread::sleep(CHECK_EVERY);
        })
        .ok();
}

// ----- Commandes -----

#[tauri::command]
pub fn backups_list(state: State<'_, AppState>) -> Vec<BackupInfo> {
    list(&state.data_dir)
}

#[tauri::command]
pub fn backup_create(state: State<'_, AppState>) -> Result<BackupInfo, String> {
    create(&state.conn(), &state.data_dir, &state.images_dir, Kind::Manuel)
}

/// Restaure une sauvegarde. Les fenêtres doivent ensuite tout recharger.
#[tauri::command]
pub fn backup_restore(state: State<'_, AppState>, id: String) -> Result<(), String> {
    restore(&mut state.conn(), &state.data_dir, &state.images_dir, &id)
}

#[tauri::command]
pub fn backup_delete(state: State<'_, AppState>, id: String) -> Result<(), String> {
    delete(&state.data_dir, &id)
}

/// Ouvre le dossier des sauvegardes dans l'explorateur.
#[tauri::command]
pub fn backups_open_folder(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = backups_dir(&state.data_dir);
    std::fs::create_dir_all(&dir).map_err(err)?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::steam::model::{Achievement, GameAchievements, Source};

    fn game(appid: u32) -> GameAchievements {
        GameAchievements {
            appid,
            game_name: None,
            source: Source::LocalCache,
            language: "french".into(),
            achievements: vec![Achievement {
                api_name: Some("A".into()),
                name: "a".into(),
                description: String::new(),
                hidden: false,
                icon_url: None,
                icon_gray_url: None,
                rarity: None,
                unlocked: Some(false),
                unlock_time: None,
                dlc_appid: None,
                progress: None,
            }],
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("grimoire-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sauvegarde_et_restauration() {
        let data = temp_dir("sauvegardes");
        let images = data.join("images");
        let mut conn = db::open(&data.join(DB_FILE)).unwrap();

        // Ancien nom du dossier repris
        std::fs::create_dir_all(data.join("sauvegardes/1-manuel")).unwrap();
        std::fs::write(data.join("sauvegardes/1-manuel").join(INFO_FILE), br#"{"created_at":1,"kind":"manuel","games":0}"#).unwrap();
        rename_old_dir(&data);
        assert_eq!(list(&data).iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["1-manuel"]);
        delete(&data, "1-manuel").unwrap();

        // Bibliothèque vide : pas de sauvegarde automatique
        assert!(auto_if_due(&conn, &data, &images).unwrap().is_none());

        db::save_game(&mut conn, &game(620), "Portal 2", Some(("covers/620.jpg", "portrait"))).unwrap();
        std::fs::create_dir_all(images.join("covers")).unwrap();
        std::fs::write(images.join("covers/620.jpg"), b"jaquette").unwrap();
        let first = auto_if_due(&conn, &data, &images).unwrap().unwrap();
        assert_eq!((first.kind, first.games), (Kind::Auto, 1));
        assert!(auto_if_due(&conn, &data, &images).unwrap().is_none(), "une par jour");

        // Le joueur supprime son jeu (et sa jaquette), puis restaure
        db::delete_game(&conn, 620).unwrap();
        std::fs::remove_file(images.join("covers/620.jpg")).unwrap();
        restore(&mut conn, &data, &images, &first.id).unwrap();
        assert_eq!(db::library_appids(&conn).unwrap(), vec![620]);
        assert_eq!(std::fs::read(images.join("covers/620.jpg")).unwrap(), b"jaquette");
        // L'état remplacé est gardé de côté
        let all = list(&data);
        assert_eq!(all.iter().map(|b| b.kind).collect::<Vec<_>>(), [Kind::AvantRestauration, Kind::Auto]);
        assert_eq!(all[0].games, 0);

        // Identifiants refusés
        assert!(restore(&mut conn, &data, &images, "../x").is_err());
        assert!(delete(&data, "inconnue").is_err());

        // Seules les 7 automatiques les plus récentes restent ; les autres sortes ne sont pas touchées
        for _ in 0..9 {
            create(&conn, &data, &images, Kind::Auto).unwrap();
        }
        prune(&data, Kind::Auto, KEEP_AUTO);
        let all = list(&data);
        assert_eq!(all.iter().filter(|b| b.kind == Kind::Auto).count(), KEEP_AUTO);
        assert_eq!(all.iter().filter(|b| b.kind == Kind::AvantRestauration).count(), 1);
        drop(conn);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Base d'une ancienne version : copiée telle quelle avant d'être migrée.
    #[test]
    fn sauvegarde_avant_mise_a_jour() {
        let data = temp_dir("avant-maj");
        let images = data.join("images");
        let old = db::schema_version() - 1;
        let mut conn = db::open_at_version(&data.join(DB_FILE), old).unwrap();
        conn.execute_batch(
            "INSERT INTO games (appid, name, source, language, imported_at, refreshed_at) VALUES (620, 'P', 'local_cache', 'french', 0, 0);",
        )
        .unwrap();

        let info = before_migration(&conn, &data, &images).unwrap().unwrap();
        assert_eq!((info.kind, info.games), (Kind::AvantMiseAJour, 1));
        db::migrate_existing(&mut conn).unwrap();
        assert!(before_migration(&conn, &data, &images).unwrap().is_none(), "base à jour : rien à faire");

        // La copie est restée à l'ancienne version
        let copy = Connection::open_with_flags(backups_dir(&data).join(&info.id).join(DB_FILE), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let v: i64 = copy.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(v as usize, old);
        drop(copy);

        // Base neuve (version 0) : pas de sauvegarde
        let fresh = db::open_unmigrated(&data.join("neuve.sqlite")).unwrap();
        assert!(!db::needs_migration(&fresh).unwrap());

        // Seules les 3 dernières sont gardées
        for _ in 0..4 {
            create(&conn, &data, &images, Kind::AvantMiseAJour).unwrap();
        }
        prune(&data, Kind::AvantMiseAJour, KEEP_BEFORE_UPDATE);
        assert_eq!(list(&data).iter().filter(|b| b.kind == Kind::AvantMiseAJour).count(), KEEP_BEFORE_UPDATE);
        drop((conn, fresh));
        let _ = std::fs::remove_dir_all(&data);
    }
}
