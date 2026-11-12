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
    // v2 : espace de travail (succès choisis, leur colonne et leur place)
    r#"
    CREATE TABLE workspace_items (
        achievement_id INTEGER PRIMARY KEY REFERENCES achievements(id) ON DELETE CASCADE,
        appid          INTEGER NOT NULL REFERENCES games(appid) ON DELETE CASCADE,
        column_name    TEXT    NOT NULL DEFAULT 'todo' CHECK (column_name IN ('todo', 'doing', 'done')),
        position       INTEGER NOT NULL,  -- ordre global ; une colonne affiche ses cartes dans cet ordre
        collapsed      INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX workspace_items_by_game ON workspace_items (appid, position);
    "#,
    // v3 : blocs des guides, rattachés à un succès (survivent au retrait de l'espace de travail)
    r#"
    CREATE TABLE blocks (
        id             INTEGER PRIMARY KEY,
        achievement_id INTEGER NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
        appid          INTEGER NOT NULL REFERENCES games(appid) ON DELETE CASCADE,
        kind           TEXT    NOT NULL CHECK (kind IN ('video', 'image', 'text', 'list', 'counter', 'timer')),
        position       INTEGER NOT NULL,
        data           TEXT    NOT NULL DEFAULT '{}',  -- JSON, forme selon `kind` (voir SPEC)
        created_at     INTEGER NOT NULL,
        updated_at     INTEGER NOT NULL
    );
    CREATE INDEX blocks_by_achievement ON blocks (achievement_id, position);
    CREATE INDEX blocks_by_game ON blocks (appid);
    "#,
    // v4 : vue Tableau, position et taille de chaque carte sur une grille de 12 colonnes
    // (NULL = pas encore placée : le tableau lui trouve une place libre)
    r#"
    ALTER TABLE workspace_items ADD COLUMN grid_x INTEGER;
    ALTER TABLE workspace_items ADD COLUMN grid_y INTEGER;
    ALTER TABLE workspace_items ADD COLUMN grid_w INTEGER;
    ALTER TABLE workspace_items ADD COLUMN grid_h INTEGER;
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
/// - un état posé à la main (coché ou décoché) est conservé tant que la source dit le contraire ;
///   dès qu'elle est d'accord, il redevient un état « Steam » ordinaire.
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
                None => (old_unlocked, old_manual, old_time),
                // La coche du joueur l'emporte tant que Steam n'est pas d'accord avec elle
                Some(s) if old_manual && s != old_unlocked => (old_unlocked, true, old_time),
                Some(true) => (true, false, a.unlock_time.or(old_time)),
                Some(false) => (false, false, None),
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

/// Coche ou décoche un succès à la main. Renvoie l'AppID du jeu concerné.
pub fn set_unlocked(conn: &Connection, id: i64, unlocked: bool) -> rusqlite::Result<u32> {
    conn.query_row(
        "UPDATE achievements
         SET unlocked = ?2, manual = 1,
             unlock_time = CASE WHEN ?2 THEN unlock_time ELSE NULL END
         WHERE id = ?1
         RETURNING appid",
        params![id, unlocked],
        |r| r.get(0),
    )
}

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

// ---------- Espace de travail ----------

/// Place d'une carte dans la vue Tableau (grille de 12 colonnes, hauteur en rangées).
#[derive(Debug, Clone, Copy, Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GridPos {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
pub struct WorkspaceItem {
    pub achievement_id: i64,
    /// Statut : `todo`, `doing` ou `done`
    pub column: String,
    pub collapsed: bool,
    /// `None` : pas encore placée dans le tableau
    #[serde(default)]
    pub grid: Option<GridPos>,
}

/// Cartes de l'espace de travail d'un jeu, dans l'ordre.
pub fn list_workspace(conn: &Connection, appid: u32) -> rusqlite::Result<Vec<WorkspaceItem>> {
    let mut stmt = conn.prepare(
        "SELECT achievement_id, column_name, collapsed, grid_x, grid_y, grid_w, grid_h FROM workspace_items
         WHERE appid = ?1 ORDER BY position",
    )?;
    let rows = stmt.query_map([appid], |r| {
        let (x, y, w, h): (Option<i64>, Option<i64>, Option<i64>, Option<i64>) =
            (r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?);
        Ok(WorkspaceItem {
            achievement_id: r.get(0)?,
            column: r.get(1)?,
            collapsed: r.get(2)?,
            grid: match (x, y, w, h) {
                (Some(x), Some(y), Some(w), Some(h)) => Some(GridPos { x, y, w, h }),
                _ => None,
            },
        })
    })?;
    rows.collect()
}

/// Remplace l'espace de travail d'un jeu par `items` (dans l'ordre donné).
/// Les succès qui n'appartiennent pas à ce jeu sont ignorés.
pub fn save_workspace(conn: &mut Connection, appid: u32, items: &[WorkspaceItem]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM workspace_items WHERE appid = ?1", [appid])?;
    {
        let mut insert = tx.prepare(
            "INSERT OR IGNORE INTO workspace_items
                 (achievement_id, appid, column_name, position, collapsed, grid_x, grid_y, grid_w, grid_h)
             SELECT id, appid, ?3, ?4, ?5, ?6, ?7, ?8, ?9 FROM achievements WHERE id = ?1 AND appid = ?2",
        )?;
        for (pos, it) in items.iter().enumerate() {
            let column = match it.column.as_str() {
                c @ ("todo" | "doing" | "done") => c,
                _ => "todo",
            };
            // Grille de 12 colonnes : on borne ce qui arrive de l'interface
            let g = it.grid.map(|g| {
                let w = g.w.clamp(1, 12);
                GridPos { x: g.x.clamp(0, 12 - w), y: g.y.max(0), w, h: g.h.clamp(1, 200) }
            });
            insert.execute(params![
                it.achievement_id,
                appid,
                column,
                pos as i64,
                it.collapsed,
                g.map(|g| g.x),
                g.map(|g| g.y),
                g.map(|g| g.w),
                g.map(|g| g.h),
            ])?;
        }
    }
    tx.commit()
}

// ---------- Blocs ----------

pub const BLOCK_KINDS: &[&str] = &["video", "image", "text", "list", "counter", "timer"];

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Block {
    pub id: i64,
    pub achievement_id: i64,
    pub kind: String,
    pub position: i64,
    pub data: serde_json::Value,
}

fn block_row(r: &rusqlite::Row) -> rusqlite::Result<Block> {
    let raw: String = r.get(4)?;
    Ok(Block {
        id: r.get(0)?,
        achievement_id: r.get(1)?,
        kind: r.get(2)?,
        position: r.get(3)?,
        data: serde_json::from_str(&raw).unwrap_or(serde_json::Value::Object(Default::default())),
    })
}

/// Tous les blocs d'un jeu, groupés par succès puis dans l'ordre.
pub fn list_blocks(conn: &Connection, appid: u32) -> rusqlite::Result<Vec<Block>> {
    let mut stmt = conn.prepare(
        "SELECT id, achievement_id, kind, position, data FROM blocks
         WHERE appid = ?1 ORDER BY achievement_id, position",
    )?;
    let rows = stmt.query_map([appid], block_row)?;
    rows.collect()
}

/// Crée un bloc sous un succès, à la fin ou à `position` (les suivants sont décalés).
pub fn create_block(
    conn: &mut Connection,
    achievement_id: i64,
    kind: &str,
    data: &serde_json::Value,
    position: Option<i64>,
) -> rusqlite::Result<Block> {
    if !BLOCK_KINDS.contains(&kind) {
        return Err(rusqlite::Error::InvalidParameterName(format!("type de bloc inconnu : {kind}")));
    }
    let tx = conn.transaction()?;
    let appid: u32 = tx.query_row("SELECT appid FROM achievements WHERE id = ?1", [achievement_id], |r| r.get(0))?;
    let end: i64 = tx.query_row(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM blocks WHERE achievement_id = ?1",
        [achievement_id],
        |r| r.get(0),
    )?;
    let pos = position.map(|p| p.clamp(0, end)).unwrap_or(end);
    tx.execute(
        "UPDATE blocks SET position = position + 1 WHERE achievement_id = ?1 AND position >= ?2",
        params![achievement_id, pos],
    )?;
    let t = now();
    tx.execute(
        "INSERT INTO blocks (achievement_id, appid, kind, position, data, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![achievement_id, appid, kind, pos, data.to_string(), t],
    )?;
    let id = tx.last_insert_rowid();
    let block = tx.query_row(
        "SELECT id, achievement_id, kind, position, data FROM blocks WHERE id = ?1",
        [id],
        block_row,
    )?;
    tx.commit()?;
    Ok(block)
}

pub fn update_block(conn: &Connection, id: i64, data: &serde_json::Value) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE blocks SET data = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, data.to_string(), now()],
    )?;
    Ok(())
}

pub fn delete_block(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM blocks WHERE id = ?1", [id])?;
    Ok(())
}

/// Nouvel ordre des blocs d'un succès. Les blocs d'autres succès sont ignorés.
pub fn reorder_blocks(conn: &mut Connection, achievement_id: i64, ids: &[i64]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare("UPDATE blocks SET position = ?3 WHERE id = ?1 AND achievement_id = ?2")?;
        for (pos, id) in ids.iter().enumerate() {
            stmt.execute(params![id, achievement_id, pos as i64])?;
        }
    }
    tx.commit()
}

/// Fichiers d'images encore utilisés par au moins un bloc (chemins relatifs à `images/`).
pub fn referenced_block_images(conn: &Connection) -> rusqlite::Result<std::collections::HashSet<String>> {
    let mut stmt = conn.prepare("SELECT json_extract(data, '$.file') FROM blocks WHERE kind = 'image'")?;
    let rows = stmt.query_map([], |r| r.get::<_, Option<String>>(0))?;
    let mut out = std::collections::HashSet::new();
    for f in rows {
        if let Some(f) = f? {
            out.insert(f);
        }
    }
    Ok(out)
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
    fn coche_manuelle_jusqu_a_accord_de_steam() {
        let mut conn = open_in_memory().unwrap();
        save_game(&mut conn, &game(vec![ach("A", Some(true)), ach("B", Some(false))]), "P", None).unwrap();
        let ids: Vec<i64> = list_achievements(&conn, 620).unwrap().iter().map(|a| a.id).collect();
        let state = |conn: &Connection| -> Vec<(bool, bool)> {
            list_achievements(conn, 620).unwrap().iter().map(|a| (a.unlocked, a.manual)).collect()
        };

        // Re-lock de A (décoché à la main) et B coché à la main
        assert_eq!(set_unlocked(&conn, ids[0], false).unwrap(), 620);
        set_unlocked(&conn, ids[1], true).unwrap();
        assert_eq!(state(&conn), vec![(false, true), (true, true)]);

        // Steam pas encore à jour : les choix du joueur tiennent
        save_game(&mut conn, &game(vec![ach("A", Some(true)), ach("B", Some(false))]), "P", None).unwrap();
        assert_eq!(state(&conn), vec![(false, true), (true, true)]);

        // Steam d'accord : l'état redevient ordinaire
        save_game(&mut conn, &game(vec![ach("A", Some(false)), ach("B", Some(true))]), "P", None).unwrap();
        assert_eq!(state(&conn), vec![(false, false), (true, false)]);
    }

    #[test]
    fn espace_de_travail() {
        let mut conn = open_in_memory().unwrap();
        save_game(&mut conn, &game(vec![ach("A", None), ach("B", None), ach("C", None)]), "P", None).unwrap();
        let ids: Vec<i64> = list_achievements(&conn, 620).unwrap().iter().map(|a| a.id).collect();
        let item = |id, col: &str| WorkspaceItem { achievement_id: id, column: col.into(), collapsed: false, grid: None };

        let mut wanted = vec![item(ids[2], "doing"), item(ids[0], "todo")];
        wanted[0].grid = Some(GridPos { x: 4, y: 0, w: 8, h: 10 });
        // Un succès d'un autre jeu et une colonne inconnue ne passent pas
        let mut sent = wanted.clone();
        sent.push(item(9999, "todo"));
        sent[1].column = "n'importe quoi".into();
        save_workspace(&mut conn, 620, &sent).unwrap();
        assert_eq!(list_workspace(&conn, 620).unwrap(), wanted);

        // Une position hors grille est ramenée dedans
        let mut odd = wanted.clone();
        odd[1].grid = Some(GridPos { x: 11, y: -3, w: 6, h: 0 });
        save_workspace(&mut conn, 620, &odd).unwrap();
        assert_eq!(list_workspace(&conn, 620).unwrap()[1].grid, Some(GridPos { x: 6, y: 0, w: 6, h: 1 }));
        save_workspace(&mut conn, 620, &wanted).unwrap();

        // L'actualisation du jeu ne touche pas à l'espace de travail
        save_game(&mut conn, &game(vec![ach("A", Some(true)), ach("C", None)]), "P", None).unwrap();
        assert_eq!(list_workspace(&conn, 620).unwrap(), wanted);

        // Supprimer le jeu vide son espace
        delete_game(&conn, 620).unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM workspace_items", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn blocs() {
        use serde_json::json;
        let mut conn = open_in_memory().unwrap();
        save_game(&mut conn, &game(vec![ach("A", None), ach("B", None)]), "P", None).unwrap();
        let ids: Vec<i64> = list_achievements(&conn, 620).unwrap().iter().map(|a| a.id).collect();

        let t = create_block(&mut conn, ids[0], "text", &json!({"markdown": "**coffre**"}), None).unwrap();
        let c = create_block(&mut conn, ids[0], "counter", &json!({"value": 3, "goal": 50, "step": 1}), None).unwrap();
        // Duplication / annulation : insertion au milieu
        let i = create_block(&mut conn, ids[0], "image", &json!({"file": "blocks/x.png"}), Some(1)).unwrap();
        assert!(create_block(&mut conn, ids[0], "pas-un-type", &json!({}), None).is_err());

        let order = |conn: &Connection| list_blocks(conn, 620).unwrap().iter().map(|b| b.id).collect::<Vec<_>>();
        assert_eq!(order(&conn), vec![t.id, i.id, c.id]);

        update_block(&conn, c.id, &json!({"value": 4, "goal": 50, "step": 1})).unwrap();
        reorder_blocks(&mut conn, ids[0], &[c.id, t.id, i.id]).unwrap();
        let list = list_blocks(&conn, 620).unwrap();
        assert_eq!(list.iter().map(|b| b.id).collect::<Vec<_>>(), vec![c.id, t.id, i.id]);
        assert_eq!(list[0].data["value"], 4);
        assert!(referenced_block_images(&conn).unwrap().contains("blocks/x.png"));

        // Actualisation et retrait de l'espace : les blocs restent
        save_game(&mut conn, &game(vec![ach("A", Some(true)), ach("B", None)]), "P", None).unwrap();
        save_workspace(&mut conn, 620, &[]).unwrap();
        assert_eq!(list_blocks(&conn, 620).unwrap().len(), 3);

        delete_block(&conn, i.id).unwrap();
        assert!(referenced_block_images(&conn).unwrap().is_empty());
        delete_game(&conn, 620).unwrap();
        assert!(list_blocks(&conn, 620).unwrap().is_empty());
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
