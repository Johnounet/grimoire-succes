//! Lecture du cache local de Steam (`appcache/stats/`). Lecture seule.
//!
//! - `UserGameStatsSchema_<appid>.bin` : schéma complet (toutes langues, succès cachés compris).
//! - `UserGameStats_<accountid>_<appid>.bin` : progression d'un compte (bits débloqués + dates).

use super::kv_binary::{self, Kv};
use super::model::{icon_url, Achievement, GameAchievements, Source};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn stats_dir(steam: &Path) -> PathBuf {
    steam.join("appcache").join("stats")
}

pub fn schema_path(steam: &Path, appid: u32) -> PathBuf {
    stats_dir(steam).join(format!("UserGameStatsSchema_{appid}.bin"))
}

pub fn progress_path(steam: &Path, account_id: u32, appid: u32) -> PathBuf {
    stats_dir(steam).join(format!("UserGameStats_{account_id}_{appid}.bin"))
}

#[derive(Debug, Clone, Serialize)]
pub struct CachedGame {
    pub appid: u32,
    pub name: Option<String>,
}

/// Jeux dont le schéma est en cache (= lancés au moins une fois sur ce PC).
pub fn list_cached_games(steam: &Path) -> Vec<CachedGame> {
    let Ok(dir) = std::fs::read_dir(stats_dir(steam)) else { return Vec::new() };
    let mut games: Vec<CachedGame> = dir
        .flatten()
        .filter_map(|e| {
            let name = e.file_name();
            let appid = name
                .to_str()?
                .strip_prefix("UserGameStatsSchema_")?
                .strip_suffix(".bin")?
                .parse()
                .ok()?;
            Some(CachedGame { appid, name: read_game_name(&e.path()) })
        })
        .collect();
    games.sort_by_key(|g| g.appid);
    games
}

/// Lit seulement le début du fichier : le nom du jeu y figure en premier.
fn read_game_name(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut buf = vec![0u8; 512];
    let n = std::fs::File::open(path).ok()?.read(&mut buf).ok()?;
    let buf = &buf[..n];
    let i = buf.windows(10).position(|w| w == b"\x01gamename\0")? + 10;
    let len = buf[i..].iter().position(|&b| b == 0)?;
    Some(String::from_utf8_lossy(&buf[i..i + len]).into_owned()).filter(|s| !s.is_empty())
}

/// Texte localisé : nœud `{english: …, french: …}` ou simple chaîne.
fn localized(node: Option<&Kv>, lang: &str) -> Option<String> {
    let node = node?;
    if let Some(s) = node.as_str() {
        return Some(s.to_string());
    }
    node.get(lang)
        .or_else(|| node.get("english"))
        .and_then(Kv::as_str)
        .map(str::to_string)
}

/// Une entrée du schéma, avec sa position (stat, bit) pour la relier à la progression.
struct SchemaEntry {
    stat_id: u32,
    bit: u32,
    achievement: Achievement,
}

fn is_achievement_stat(stat: &Kv) -> bool {
    match stat.get("type") {
        Some(Kv::Str(s)) => s.eq_ignore_ascii_case("ACHIEVEMENTS") || s.trim() == "4",
        Some(v) => v.as_i64() == Some(4),
        None => stat.get("bits").is_some(),
    }
}

fn parse_schema(root: &Kv, appid: u32, lang: &str) -> (Option<String>, Vec<SchemaEntry>) {
    let game = root.get(&appid.to_string()).unwrap_or(root);
    let game_name = game.get("gamename").and_then(Kv::as_str).map(str::to_string);

    let mut entries = Vec::new();
    for (stat_key, stat) in game.get("stats").map(Kv::children).unwrap_or_default() {
        if !is_achievement_stat(stat) {
            continue;
        }
        let Ok(stat_id) = stat_key.parse::<u32>() else { continue };
        for (bit_key, b) in stat.get("bits").map(Kv::children).unwrap_or_default() {
            let Some(bit) = b
                .get("bit")
                .and_then(Kv::as_i64)
                .and_then(|v| u32::try_from(v).ok())
                .or_else(|| bit_key.parse().ok())
            else {
                continue;
            };
            let display = b.get("display");
            let field = |k: &str| display.and_then(|d| d.get(k));
            let api_name = b.get("name").and_then(Kv::as_str).map(str::to_string);
            entries.push(SchemaEntry {
                stat_id,
                bit,
                achievement: Achievement {
                    name: localized(field("name"), lang)
                        .or_else(|| api_name.clone())
                        .unwrap_or_default(),
                    description: localized(field("desc"), lang).unwrap_or_default(),
                    api_name,
                    hidden: field("hidden").and_then(Kv::as_bool).unwrap_or(false),
                    icon_url: field("icon").and_then(Kv::as_str).and_then(|f| icon_url(appid, f)),
                    icon_gray_url: field("icon_gray")
                        .and_then(Kv::as_str)
                        .and_then(|f| icon_url(appid, f)),
                    rarity: None,
                    unlocked: None,
                    unlock_time: None,
                    dlc_appid: None,
                },
            });
        }
    }
    (game_name, entries)
}

/// État débloqué par (stat, bit) : `data` est un champ de bits, `AchievementTimes` les dates.
fn parse_progress(root: &Kv) -> HashMap<(u32, u32), i64> {
    let cache = root.get("cache").unwrap_or(root);
    let mut out = HashMap::new();
    for (stat_key, stat) in cache.children() {
        let Ok(stat_id) = stat_key.parse::<u32>() else { continue };
        let bits = stat.get("data").and_then(Kv::as_i64).unwrap_or(0) as u32;
        let times = stat.get("AchievementTimes");
        for bit in 0..32 {
            if bits & (1 << bit) != 0 {
                let t = times
                    .and_then(|t| t.get(&bit.to_string()))
                    .and_then(Kv::as_i64)
                    .unwrap_or(0);
                out.insert((stat_id, bit), t);
            }
        }
    }
    out
}

/// Charge les succès d'un jeu depuis le cache local.
/// `account_id = None` : schéma seul, état inconnu.
pub fn load(steam: &Path, appid: u32, account_id: Option<u32>, lang: &str) -> Result<GameAchievements, String> {
    let path = schema_path(steam, appid);
    let bytes = std::fs::read(&path)
        .map_err(|_| format!("jeu {appid} absent du cache local (jamais lancé sur ce PC ?)"))?;
    let root = kv_binary::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let (game_name, entries) = parse_schema(&root, appid, lang);
    if entries.is_empty() {
        return Err(format!("aucun succès dans le schéma local du jeu {appid}"));
    }

    let progress = match account_id {
        Some(acc) => match std::fs::read(progress_path(steam, acc, appid)) {
            Ok(b) => Some(parse_progress(&kv_binary::parse(&b).map_err(|e| e.to_string())?)),
            Err(_) => None,
        },
        None => None,
    };

    let achievements = entries
        .into_iter()
        .map(|e| {
            let mut a = e.achievement;
            if let Some(p) = &progress {
                let t = p.get(&(e.stat_id, e.bit));
                a.unlocked = Some(t.is_some());
                a.unlock_time = t.copied().filter(|&t| t > 0);
            }
            a
        })
        .collect();

    Ok(GameAchievements {
        appid,
        game_name,
        source: Source::LocalCache,
        language: lang.to_string(),
        achievements,
    })
}
