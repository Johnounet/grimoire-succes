//! Base SQLite locale : un seul fichier dans le dossier de données de l'application.
//!
//! Le schéma est versionné par `PRAGMA user_version` : chaque entrée de `MIGRATIONS`
//! fait passer la base à la version suivante. Ne jamais modifier une migration publiée,
//! toujours en ajouter une nouvelle.

use crate::steam::model::{Achievement, GameAchievements, Source};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

const MIGRATIONS: &[&str] = &[
    // v1 : bibliothèque de jeux, succès, réglages
    r#"
    CREATE TABLE games (
        appid            INTEGER PRIMARY KEY,
        name             TEXT    NOT NULL,
        cover_file       TEXT,             -- relatif au dossier images/
        cover_kind       TEXT,             -- 'portrait' (600x900) ou 'header' (repli)
        source           TEXT    NOT NULL, -- local_cache | public_profile | public_page
        language         TEXT    NOT NULL,
        imported_at      INTEGER NOT NULL,
        refreshed_at     INTEGER NOT NULL,
        completed_at     INTEGER,
        playtime_minutes INTEGER,
        sort_order       INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE achievements (
        id            INTEGER PRIMARY KEY,
        appid         INTEGER NOT NULL REFERENCES games(appid) ON DELETE CASCADE,
        key           TEXT    NOT NULL,  -- api_name, ou 'name:<nom>' si l'ID technique manque
        api_name      TEXT,
        name          TEXT    NOT NULL,
        description   TEXT    NOT NULL DEFAULT '',
        hidden        INTEGER NOT NULL DEFAULT 0,
        icon_url      TEXT,
        icon_gray_url TEXT,
        rarity        REAL,
        unlocked      INTEGER NOT NULL DEFAULT 0,
        unlock_time   INTEGER,
        manual        INTEGER NOT NULL DEFAULT 0, -- état posé à la main par le joueur
        dlc_appid     INTEGER,
        steam_order   INTEGER NOT NULL,
        removed       INTEGER NOT NULL DEFAULT 0, -- disparu de Steam : gardé pour ses guides
        UNIQUE (appid, key)
    );

    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
];

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    migrate(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&mut conn)?;
    Ok(conn)
}

fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn source_str(s: Source) -> &'static str {
    match s {
        Source::LocalCache => "local_cache",
        Source::PublicProfile => "public_profile",
        Source::PublicPage => "public_page",
    }
}

fn achievement_key(a: &Achievement) -> String {
    a.api_name.clone().unwrap_or_else(|| format!("name:{}", a.name))
}

// ---------- Réglages ----------

pub fn get_setting(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()
}

#[allow(dead_code)] // utilisé par les Réglages (étapes suivantes)
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

// ---------- Jeux ----------

#[derive(Debug, Clone, Serialize)]
pub struct GameSummary {
    pub appid: u32,
    pub name: String,
    pub cover_file: Option<String>,
    pub cover_kind: Option<String>,
    pub source: String,
    pub total: u32,
    pub unlocked: u32,
    pub has_api_names: bool,
    pub refreshed_at: i64,
}

const SUMMARY_SQL: &str = "
    SELECT g.appid, g.name, g.cover_file, g.cover_kind, g.source, g.refreshed_at,
           COUNT(a.id),
           COALESCE(SUM(a.unlocked), 0),
           COUNT(a.id) = COUNT(a.api_name)
    FROM games g
    LEFT JOIN achievements a ON a.appid = g.appid AND a.removed = 0";

fn summary_row(r: &rusqlite::Row) -> rusqlite::Result<GameSummary> {
    Ok(GameSummary {
        appid: r.get(0)?,
        name: r.get(1)?,
        cover_file: r.get(2)?,
        cover_kind: r.get(3)?,
        source: r.get(4)?,
        refreshed_at: r.get(5)?,
        total: r.get(6)?,
        unlocked: r.get(7)?,
        has_api_names: r.get(8)?,
    })
}

pub fn list_games(conn: &Connection) -> rusqlite::Result<Vec<GameSummary>> {
    let mut stmt = conn.prepare(&format!("{SUMMARY_SQL} GROUP BY g.appid ORDER BY g.sort_order, g.name"))?;
    let rows = stmt.query_map([], summary_row)?;
    rows.collect()
}

pub fn get_game(conn: &Connection, appid: u32) -> rusqlite::Result<Option<GameSummary>> {
    conn.query_row(&format!("{SUMMARY_SQL} WHERE g.appid = ?1 GROUP BY g.appid"), [appid], summary_row)
        .optional()
}

pub fn game_exists(conn: &Connection, appid: u32) -> rusqlite::Result<bool> {
    conn.query_row("SELECT 1 FROM games WHERE appid = ?1", [appid], |_| Ok(()))
        .optional()
        .map(|o| o.is_some())
}

pub fn library_appids(conn: &Connection) -> rusqlite::Result<Vec<u32>> {
    let mut stmt = conn.prepare("SELECT appid FROM games")?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    rows.collect()
}

/// Crée ou met à jour un jeu et ses succès.
///
/// Règles d'actualisation (SPEC, « Règles de sauvegarde ») :
/// - les guides et étiquettes ne sont jamais touchés (ils référencent l'`id` du succès, qui reste stable) ;
/// - un succès disparu de Steam est marqué `removed`, pas supprimé ;
/// - un succès coché à la main reste coché si la source dit « non obtenu » ou ne sait pas.
pub fn save_game(
    conn: &mut Connection,
    data: &GameAchievements,
    name: &str,
    cover: Option<(&str, &str)>,
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let t = now();

    let next_order: i64 = tx.query_row("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM games", [], |r| r.get(0))?;
    tx.execute(
        "INSERT INTO games (appid, name, cover_file, cover_kind, source, language,
                            imported_at, refreshed_at, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8)
         ON CONFLICT(appid) DO UPDATE SET
            name = excluded.name,
            cover_file = COALESCE(excluded.cover_file, games.cover_file),
            cover_kind = COALESCE(excluded.cover_kind, games.cover_kind),
            source = excluded.source,
            language = excluded.language,
            refreshed_at = excluded.refreshed_at",
        params![
            data.appid,
            name,
            cover.map(|c| c.0),
            cover.map(|c| c.1),
            source_str(data.source),
            data.language,
            t,
            next_order
        ],
    )?;

    // État actuel, pour respecter les coches manuelles
    let existing: HashMap<String, (bool, bool, Option<i64>)> = {
        let mut stmt = tx.prepare("SELECT key, unlocked, manual, unlock_time FROM achievements WHERE appid = ?1")?;
        let rows = stmt.query_map([data.appid], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?, r.get(3)?))))?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    tx.execute("UPDATE achievements SET removed = 1 WHERE appid = ?1", [data.appid])?;

    {
        let mut upsert = tx.prepare(
            "INSERT INTO achievements (appid, key, api_name, name, description, hidden, icon_url,
                                       icon_gray_url, rarity, unlocked, unlock_time, manual,
                                       dlc_appid, steam_order, removed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 0)
             ON CONFLICT(appid, key) DO UPDATE SET
                api_name = excluded.api_name, name = excluded.name,
                description = excluded.description, hidden = excluded.hidden,
                icon_url = excluded.icon_url, icon_gray_url = excluded.icon_gray_url,
                rarity = COALESCE(excluded.rarity, achievements.rarity),
                unlocked = excluded.unlocked, unlock_time = excluded.unlock_time,
                manual = excluded.manual, dlc_appid = excluded.dlc_appid,
                steam_order = excluded.steam_order, removed = 0",
        )?;

        for (order, a) in data.achievements.iter().enumerate() {
            let key = achievement_key(a);
            let (old_unlocked, old_manual, old_time) = existing.get(&key).copied().unwrap_or((false, false, None));
            let (unlocked, manual, time) = match a.unlocked {
                Some(true) => (true, false, a.unlock_time.or(old_time)),
                Some(false) if old_manual => (old_unlocked, true, old_time),
                Some(false) => (false, false, None),
                None => (old_unlocked, old_manual, old_time),
            };
            upsert.execute(params![
                data.appid,
                key,
                a.api_name,
                a.name,
                a.description,
                a.hidden,
                a.icon_url,
                a.icon_gray_url,
                a.rarity,
                unlocked,
                time,
                manual,
                a.dlc_appid,
                order as i64,
            ])?;
        }
    }

    tx.commit()
}

pub fn delete_game(conn: &Connection, appid: u32) -> rusqlite::Result<Option<String>> {
    let cover: Option<String> = conn
        .query_row("SELECT cover_file FROM games WHERE appid = ?1", [appid], |r| r.get(0))
        .optional()?
        .flatten();
    conn.execute("DELETE FROM games WHERE appid = ?1", [appid])?;
    Ok(cover)
}

// ---------- Succès ----------

#[derive(Debug, Clone, Serialize)]
pub struct AchievementRow {
    pub id: i64,
    pub api_name: Option<String>,
    pub name: String,
    pub description: String,
    pub hidden: bool,
    pub icon_url: Option<String>,
    pub icon_gray_url: Option<String>,
    pub rarity: Option<f32>,
    pub unlocked: bool,
    pub unlock_time: Option<i64>,
    pub manual: bool,
    pub dlc_appid: Option<u32>,
    pub removed: bool,
}

pub fn list_achievements(conn: &Connection, appid: u32) -> rusqlite::Result<Vec<AchievementRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, api_name, name, description, hidden, icon_url, icon_gray_url, rarity,
                unlocked, unlock_time, manual, dlc_appid, removed
         FROM achievements WHERE appid = ?1 ORDER BY removed, steam_order",
    )?;
    let rows = stmt.query_map([appid], |r| {
        Ok(AchievementRow {
            id: r.get(0)?,
            api_name: r.get(1)?,
            name: r.get(2)?,
            description: r.get(3)?,
            hidden: r.get(4)?,
            icon_url: r.get(5)?,
            icon_gray_url: r.get(6)?,
            rarity: r.get(7)?,
            unlocked: r.get(8)?,
            unlock_time: r.get(9)?,
            manual: r.get(10)?,
            dlc_appid: r.get(11)?,
            removed: r.get(12)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ach(api: &str, unlocked: Option<bool>) -> Achievement {
        Achievement {
            api_name: Some(api.into()),
            name: api.to_lowercase(),
            description: String::new(),
            hidden: false,
            icon_url: None,
            icon_gray_url: None,
            rarity: Some(10.0),
            unlocked,
            unlock_time: unlocked.filter(|&u| u).map(|_| 1000),
            dlc_appid: None,
        }
    }

    fn game(achievements: Vec<Achievement>) -> GameAchievements {
        GameAchievements {
            appid: 620,
            game_name: Some("Portal 2".into()),
            source: Source::LocalCache,
            language: "french".into(),
            achievements,
        }
    }

    #[test]
    fn migrations_idempotentes() {
        let mut conn = open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        let v: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(v as usize, MIGRATIONS.len());
    }

    #[test]
    fn import_puis_actualisation() {
        let mut conn = open_in_memory().unwrap();
        save_game(&mut conn, &game(vec![ach("A", Some(true)), ach("B", Some(false))]), "Portal 2", None).unwrap();

        let g = get_game(&conn, 620).unwrap().unwrap();
        assert_eq!((g.total, g.unlocked), (2, 1));
        let id_b = list_achievements(&conn, 620).unwrap()[1].id;

        // Le joueur coche B à la main, puis Steam retire A et ajoute C
        conn.execute("UPDATE achievements SET unlocked = 1, manual = 1 WHERE key = 'B'", []).unwrap();
        save_game(&mut conn, &game(vec![ach("B", Some(false)), ach("C", None)]), "Portal 2", None).unwrap();

        let list = list_achievements(&conn, 620).unwrap();
        let b = list.iter().find(|a| a.api_name.as_deref() == Some("B")).unwrap();
        assert_eq!(b.id, id_b, "l'id reste stable pour ne pas perdre les guides");
        assert!(b.unlocked && b.manual, "la coche manuelle est conservée");
        assert!(list.iter().find(|a| a.api_name.as_deref() == Some("A")).unwrap().removed);

        let g = get_game(&conn, 620).unwrap().unwrap();
        assert_eq!((g.total, g.unlocked), (2, 1), "A retiré n'est plus compté");
        assert_eq!(list_games(&conn).unwrap().len(), 1);
    }

    #[test]
    fn suppression_en_cascade() {
        let mut conn = open_in_memory().unwrap();
        save_game(&mut conn, &game(vec![ach("A", None)]), "Portal 2", Some(("covers/620.jpg", "portrait"))).unwrap();
        assert_eq!(delete_game(&conn, 620).unwrap().as_deref(), Some("covers/620.jpg"));
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM achievements", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }
}
