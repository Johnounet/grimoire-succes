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
    // v5 : étiquettes (communes à tous les jeux, posées sur les succès) et cartes épinglées
    // au mode compact
    r#"
    CREATE TABLE tags (
        id       INTEGER PRIMARY KEY,
        name     TEXT    NOT NULL UNIQUE COLLATE NOCASE,
        color    TEXT    NOT NULL,  -- #rrggbb
        position INTEGER NOT NULL
    );
    CREATE TABLE achievement_tags (
        achievement_id INTEGER NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
        tag_id         INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
        PRIMARY KEY (achievement_id, tag_id)
    );
    CREATE INDEX achievement_tags_by_tag ON achievement_tags (tag_id);
    INSERT INTO tags (name, color, position) VALUES
        ('ratable', '#ef5b5b', 0),
        ('grind', '#f2994a', 1),
        ('en ligne', '#4f7cff', 2),
        ('difficile', '#b07cff', 3),
        ('cumulable', '#4cc38a', 4),
        ('DLC', '#f2c14e', 5);
    ALTER TABLE workspace_items ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
    "#,
    // v6 : dernière taille de chaque carte du tableau, gardée quand la carte est retirée de l'espace
    // de travail, pour qu'elle la retrouve si on l'y remet
    r#"
    CREATE TABLE card_sizes (
        achievement_id INTEGER PRIMARY KEY REFERENCES achievements(id) ON DELETE CASCADE,
        appid          INTEGER NOT NULL REFERENCES games(appid) ON DELETE CASCADE,
        w              INTEGER NOT NULL,
        h              INTEGER NOT NULL
    );
    CREATE INDEX card_sizes_by_game ON card_sizes (appid);
    INSERT INTO card_sizes (achievement_id, appid, w, h)
        SELECT achievement_id, appid, grid_w, grid_h FROM workspace_items
        WHERE grid_w IS NOT NULL AND grid_h IS NOT NULL;
    "#,
    // v7 : 100 %. `completed_at` (colonne de la v1) est désormais tenue à jour ; `celebrated` retient que
    // la célébration a déjà été montrée (une seule fois par jeu, rejouable depuis la vitrine)
    r#"
    ALTER TABLE games ADD COLUMN celebrated INTEGER NOT NULL DEFAULT 0;
    UPDATE games SET completed_at = COALESCE(
            (SELECT MAX(unlock_time) FROM achievements a WHERE a.appid = games.appid AND a.removed = 0),
            CAST(strftime('%s', 'now') AS INTEGER))
        WHERE EXISTS (SELECT 1 FROM achievements a WHERE a.appid = games.appid AND a.removed = 0)
          AND NOT EXISTS (SELECT 1 FROM achievements a WHERE a.appid = games.appid AND a.removed = 0 AND a.unlocked = 0);
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
    /// Date du 100 % (secondes Unix), `None` tant que le jeu n'est pas terminé
    pub completed_at: Option<i64>,
    pub playtime_minutes: Option<u32>,
    /// La célébration du 100 % a déjà été montrée
    pub celebrated: bool,
}

const SUMMARY_SQL: &str = "
    SELECT g.appid, g.name, g.cover_file, g.cover_kind, g.source, g.refreshed_at,
           COUNT(a.id),
           COALESCE(SUM(a.unlocked), 0),
           COUNT(a.id) = COUNT(a.api_name),
           g.completed_at, g.playtime_minutes, g.celebrated
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
        completed_at: r.get(9)?,
        playtime_minutes: r.get(10)?,
        celebrated: r.get(11)?,
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

    update_completion(&tx, data.appid, None)?;
    tx.commit()
}

/// Tient à jour la date du 100 % après un changement d'état des succès.
///
/// Jeu terminé et pas encore daté : `when` (coche du joueur), sinon la date du dernier succès obtenu
/// d'après Steam, sinon maintenant. Jeu qui n'est plus terminé (re-lock, nouveaux succès) : plus de date.
fn update_completion(conn: &Connection, appid: u32, when: Option<i64>) -> rusqlite::Result<()> {
    let (total, missing, last): (i64, i64, Option<i64>) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(unlocked = 0), 0), MAX(unlock_time)
         FROM achievements WHERE appid = ?1 AND removed = 0",
        [appid],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    if total > 0 && missing == 0 {
        conn.execute(
            "UPDATE games SET completed_at = ?2 WHERE appid = ?1 AND completed_at IS NULL",
            params![appid, when.or(last).unwrap_or_else(now)],
        )?;
    } else {
        conn.execute("UPDATE games SET completed_at = NULL WHERE appid = ?1", [appid])?;
    }
    Ok(())
}

/// Temps de jeu lu dans Steam. Un temps inconnu n'efface pas le dernier connu.
pub fn set_playtime(conn: &Connection, appid: u32, minutes: Option<u32>) -> rusqlite::Result<()> {
    if let Some(m) = minutes {
        conn.execute("UPDATE games SET playtime_minutes = ?2 WHERE appid = ?1", params![appid, m])?;
    }
    Ok(())
}

pub fn mark_celebrated(conn: &Connection, appid: u32) -> rusqlite::Result<()> {
    conn.execute("UPDATE games SET celebrated = 1 WHERE appid = ?1", [appid])?;
    Ok(())
}

/// Succès obtenu le plus rare d'un jeu (rareté connue seulement).
pub fn rarest_unlocked(conn: &Connection, appid: u32) -> rusqlite::Result<Option<AchievementRow>> {
    Ok(list_achievements(conn, appid)?
        .into_iter()
        .filter(|a| a.unlocked && !a.removed && a.rarity.is_some())
        .min_by(|a, b| a.rarity.partial_cmp(&b.rarity).unwrap_or(std::cmp::Ordering::Equal)))
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
    let appid = conn.query_row(
        "UPDATE achievements
         SET unlocked = ?2, manual = 1,
             unlock_time = CASE WHEN ?2 THEN unlock_time ELSE NULL END
         WHERE id = ?1
         RETURNING appid",
        params![id, unlocked],
        |r| r.get(0),
    )?;
    // Le dernier succès coché à la main date le 100 % d'aujourd'hui
    update_completion(conn, appid, Some(now()))?;
    Ok(appid)
}

/// Décoche tous les succès d'un jeu (après un reset complet lancé par le joueur dans Steam).
pub fn lock_all(conn: &Connection, appid: u32) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE achievements SET unlocked = 0, manual = 1, unlock_time = NULL WHERE appid = ?1 AND unlocked = 1",
        [appid],
    )?;
    update_completion(conn, appid, None)
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
    /// Affichée dans le mode compact
    #[serde(default)]
    pub pinned: bool,
}

/// Cartes de l'espace de travail d'un jeu, dans l'ordre.
pub fn list_workspace(conn: &Connection, appid: u32) -> rusqlite::Result<Vec<WorkspaceItem>> {
    let mut stmt = conn.prepare(
        "SELECT achievement_id, column_name, collapsed, grid_x, grid_y, grid_w, grid_h, pinned FROM workspace_items
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
            pinned: r.get(7)?,
        })
    })?;
    rows.collect()
}

/// Taille mémorisée d'une carte du tableau (y compris des cartes retirées de l'espace de travail).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CardSize {
    pub achievement_id: i64,
    pub w: i64,
    pub h: i64,
}

pub fn card_sizes(conn: &Connection, appid: u32) -> rusqlite::Result<Vec<CardSize>> {
    let mut stmt = conn.prepare("SELECT achievement_id, w, h FROM card_sizes WHERE appid = ?1")?;
    let rows = stmt.query_map([appid], |r| Ok(CardSize { achievement_id: r.get(0)?, w: r.get(1)?, h: r.get(2)? }))?;
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
                 (achievement_id, appid, column_name, position, collapsed, grid_x, grid_y, grid_w, grid_h, pinned)
             SELECT id, appid, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10 FROM achievements WHERE id = ?1 AND appid = ?2",
        )?;
        let mut remember = tx.prepare(
            "INSERT INTO card_sizes (achievement_id, appid, w, h)
             SELECT id, appid, ?3, ?4 FROM achievements WHERE id = ?1 AND appid = ?2
             ON CONFLICT (achievement_id) DO UPDATE SET w = excluded.w, h = excluded.h",
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
                it.pinned,
            ])?;
            if let Some(g) = g {
                remember.execute(params![it.achievement_id, appid, g.w, g.h])?;
            }
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

/// Un bloc et le succès qui le porte (compteur ou chrono « actif », qui peut être d'un autre jeu).
#[derive(Debug, Clone, Serialize)]
pub struct BlockInfo {
    pub block: Block,
    pub appid: u32,
    pub achievement_name: String,
}

pub fn get_block(conn: &Connection, id: i64) -> rusqlite::Result<Option<BlockInfo>> {
    conn.query_row(
        "SELECT b.id, b.achievement_id, b.kind, b.position, b.data, b.appid, a.name
         FROM blocks b JOIN achievements a ON a.id = b.achievement_id WHERE b.id = ?1",
        [id],
        |r| Ok(BlockInfo { block: block_row(r)?, appid: r.get(5)?, achievement_name: r.get(6)? }),
    )
    .optional()
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

// ---------- Étiquettes ----------

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
}

/// Nom nettoyé et couleur `#rrggbb` ; sinon un message pour le joueur.
fn check_tag(name: &str, color: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Le nom de l'étiquette est vide.".into());
    }
    if name.chars().count() > 40 {
        return Err("Nom d'étiquette trop long (40 caractères au plus).".into());
    }
    let ok = color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !ok {
        return Err(format!("Couleur invalide : {color}"));
    }
    Ok(name.to_string())
}

fn tag_error(e: rusqlite::Error) -> String {
    match e {
        rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation => {
            "Une étiquette porte déjà ce nom.".into()
        }
        e => e.to_string(),
    }
}

pub fn list_tags(conn: &Connection) -> rusqlite::Result<Vec<Tag>> {
    let mut stmt = conn.prepare("SELECT id, name, color FROM tags ORDER BY position, id")?;
    let rows = stmt.query_map([], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? }))?;
    rows.collect()
}

pub fn create_tag(conn: &Connection, name: &str, color: &str) -> Result<Tag, String> {
    let name = check_tag(name, color)?;
    conn.query_row(
        "INSERT INTO tags (name, color, position) VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM tags))
         RETURNING id, name, color",
        params![name, color],
        |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? }),
    )
    .map_err(tag_error)
}

pub fn update_tag(conn: &Connection, id: i64, name: &str, color: &str) -> Result<(), String> {
    let name = check_tag(name, color)?;
    conn.execute("UPDATE tags SET name = ?2, color = ?3 WHERE id = ?1", params![id, name, color])
        .map_err(tag_error)?;
    Ok(())
}

/// Supprime une étiquette partout (elle disparaît des succès de tous les jeux).
pub fn delete_tag(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM tags WHERE id = ?1", [id])?;
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct AchievementTag {
    pub achievement_id: i64,
    pub tag_id: i64,
}

/// Étiquettes posées sur les succès d'un jeu.
pub fn list_achievement_tags(conn: &Connection, appid: u32) -> rusqlite::Result<Vec<AchievementTag>> {
    let mut stmt = conn.prepare(
        "SELECT t.achievement_id, t.tag_id FROM achievement_tags t
         JOIN achievements a ON a.id = t.achievement_id
         WHERE a.appid = ?1 ORDER BY t.achievement_id, t.tag_id",
    )?;
    let rows = stmt.query_map([appid], |r| Ok(AchievementTag { achievement_id: r.get(0)?, tag_id: r.get(1)? }))?;
    rows.collect()
}

pub fn set_achievement_tag(conn: &Connection, achievement_id: i64, tag_id: i64, on: bool) -> rusqlite::Result<()> {
    if on {
        conn.execute(
            "INSERT OR IGNORE INTO achievement_tags (achievement_id, tag_id) VALUES (?1, ?2)",
            params![achievement_id, tag_id],
        )?;
    } else {
        conn.execute(
            "DELETE FROM achievement_tags WHERE achievement_id = ?1 AND tag_id = ?2",
            params![achievement_id, tag_id],
        )?;
    }
    Ok(())
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
        let item = |id, col: &str| WorkspaceItem { achievement_id: id, column: col.into(), collapsed: false, grid: None, pinned: false };

        let mut wanted = vec![item(ids[2], "doing"), item(ids[0], "todo")];
        wanted[0].grid = Some(GridPos { x: 4, y: 0, w: 8, h: 10 });
        wanted[1].pinned = true;
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

        // Une carte retirée garde sa dernière taille, pour la retrouver si on la remet
        save_workspace(&mut conn, 620, &wanted[1..]).unwrap();
        assert!(card_sizes(&conn, 620).unwrap().contains(&CardSize { achievement_id: ids[2], w: 8, h: 10 }));
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
    fn etiquettes() {
        let mut conn = open_in_memory().unwrap();
        save_game(&mut conn, &game(vec![ach("A", None), ach("B", None)]), "P", None).unwrap();
        let ids: Vec<i64> = list_achievements(&conn, 620).unwrap().iter().map(|a| a.id).collect();

        // Étiquettes proposées au départ
        let tags = list_tags(&conn).unwrap();
        let names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["ratable", "grind", "en ligne", "difficile", "cumulable", "DLC"]);

        let t = create_tag(&conn, "  speedrun ", "#123abc").unwrap();
        assert_eq!(t.name, "speedrun");
        assert!(create_tag(&conn, "GRIND", "#000000").is_err(), "nom déjà pris (sans tenir compte de la casse)");
        assert!(create_tag(&conn, "x", "rouge").is_err());
        assert!(create_tag(&conn, " ", "#000000").is_err());
        update_tag(&conn, t.id, "course", "#ffffff").unwrap();
        assert!(update_tag(&conn, t.id, "ratable", "#ffffff").is_err());

        set_achievement_tag(&conn, ids[0], t.id, true).unwrap();
        set_achievement_tag(&conn, ids[0], t.id, true).unwrap();
        set_achievement_tag(&conn, ids[1], tags[0].id, true).unwrap();
        assert_eq!(list_achievement_tags(&conn, 620).unwrap().len(), 2);
        set_achievement_tag(&conn, ids[1], tags[0].id, false).unwrap();

        // L'actualisation garde les étiquettes ; supprimer l'étiquette la retire partout
        save_game(&mut conn, &game(vec![ach("A", Some(true)), ach("B", None)]), "P", None).unwrap();
        assert_eq!(
            list_achievement_tags(&conn, 620).unwrap(),
            vec![AchievementTag { achievement_id: ids[0], tag_id: t.id }]
        );
        delete_tag(&conn, t.id).unwrap();
        assert!(list_achievement_tags(&conn, 620).unwrap().is_empty());
    }

    #[test]
    fn reset_complet_et_bloc_actif() {
        use serde_json::json;
        let mut conn = open_in_memory().unwrap();
        save_game(&mut conn, &game(vec![ach("A", Some(true)), ach("B", Some(true))]), "Portal 2", None).unwrap();
        lock_all(&conn, 620).unwrap();
        let list = list_achievements(&conn, 620).unwrap();
        assert!(list.iter().all(|a| !a.unlocked && a.manual && a.unlock_time.is_none()));

        let c = create_block(&mut conn, list[0].id, "counter", &json!({"value": 1}), None).unwrap();
        let info = get_block(&conn, c.id).unwrap().unwrap();
        assert_eq!((info.appid, info.achievement_name.as_str(), info.block.id), (620, "a", c.id));
        assert!(get_block(&conn, 9999).unwrap().is_none());
    }

    #[test]
    fn cent_pour_cent() {
        let mut conn = open_in_memory().unwrap();
        let mut b = ach("B", Some(true));
        b.unlock_time = Some(2000);
        b.rarity = Some(2.5);
        save_game(&mut conn, &game(vec![ach("A", Some(true)), ach("C", Some(false))]), "P", None).unwrap();
        let g = get_game(&conn, 620).unwrap().unwrap();
        assert_eq!((g.completed_at, g.celebrated), (None, false));

        // Terminé dans Steam : daté du dernier succès obtenu
        save_game(&mut conn, &game(vec![ach("A", Some(true)), b.clone()]), "P", None).unwrap();
        assert_eq!(get_game(&conn, 620).unwrap().unwrap().completed_at, Some(2000));
        assert_eq!(rarest_unlocked(&conn, 620).unwrap().unwrap().api_name.as_deref(), Some("B"));

        // Un succès décoché : plus terminé ; recoché à la main : daté d'aujourd'hui
        let id_a = list_achievements(&conn, 620).unwrap()[0].id;
        set_unlocked(&conn, id_a, false).unwrap();
        assert_eq!(get_game(&conn, 620).unwrap().unwrap().completed_at, None);
        set_unlocked(&conn, id_a, true).unwrap();
        assert!(get_game(&conn, 620).unwrap().unwrap().completed_at.unwrap() > 1_700_000_000);

        // Célébration retenue ; temps de jeu inconnu n'efface pas le connu
        mark_celebrated(&conn, 620).unwrap();
        set_playtime(&conn, 620, Some(90)).unwrap();
        set_playtime(&conn, 620, None).unwrap();
        let g = get_game(&conn, 620).unwrap().unwrap();
        assert_eq!((g.celebrated, g.playtime_minutes), (true, Some(90)));

        // Reset complet
        lock_all(&conn, 620).unwrap();
        assert_eq!(get_game(&conn, 620).unwrap().unwrap().completed_at, None);
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
