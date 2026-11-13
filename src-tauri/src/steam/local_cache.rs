//! Lecture du cache local de Steam (`appcache/stats/`). Lecture seule.
//!
//! - `UserGameStatsSchema_<appid>.bin` : schéma complet (toutes langues, succès cachés compris).
//! - `UserGameStats_<accountid>_<appid>.bin` : progression d'un compte (bits débloqués + dates,
//!   valeurs des statistiques du jeu).
//!
//! Progression d'un succès (« 37 / 50 ») : dans le schéma, le bit du succès peut porter
//! `progress { min_val, max_val, value { operation = "statvalue", operand1 = <nom de stat> } }` ;
//! la stat est décrite dans `stats` (`name`, `type` INT / FLOAT) et sa valeur est le `data` de la même
//! clé dans le fichier du compte (bits d'un f32 pour une stat FLOAT).

use super::kv_binary::{self, Kv};
use super::model::{icon_url, Achievement, GameAchievements, Progress, Source};
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

/// Jeux dont le schéma est en cache (= lancés au moins une fois sur ce PC). Nom officiel du manifeste
/// d'installation s'il existe, sinon celui du schéma (parfois court ou nom de code).
pub fn list_cached_games(steam: &Path) -> Vec<CachedGame> {
    let Ok(dir) = std::fs::read_dir(stats_dir(steam)) else { return Vec::new() };
    let mut installed = super::manifests::installed_names(steam);
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
            let name = installed.remove(&appid).or_else(|| read_game_name(&e.path()));
            Some(CachedGame { appid, name })
        })
        .collect();
    games.sort_by_key(|g| g.appid);
    games
}

/// Nom du jeu inscrit dans le schéma. Il est tantôt au début du fichier, tantôt à la fin : on lit
/// ces deux morceaux avant de se résoudre à lire tout le fichier (quelques schémas, jusqu'à plusieurs Mo).
fn read_game_name(path: &Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    const PART: u64 = 2048;
    let mut file = std::fs::File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    let mut head = vec![0u8; PART.min(size) as usize];
    file.read_exact(&mut head).ok()?;
    if let Some(name) = find_game_name(&head) {
        return Some(name);
    }
    if size > PART {
        let mut tail = vec![0u8; PART.min(size) as usize];
        file.seek(SeekFrom::End(-(tail.len() as i64))).ok()?;
        file.read_exact(&mut tail).ok()?;
        if let Some(name) = find_game_name(&tail) {
            return Some(name);
        }
    }
    find_game_name(&std::fs::read(path).ok()?)
}

/// Valeur de la chaîne `gamename` dans un morceau de fichier KeyValues binaire.
fn find_game_name(buf: &[u8]) -> Option<String> {
    let i = buf.windows(10).position(|w| w == b"\x01gamename\0")? + 10;
    let len = buf[i..].iter().position(|&b| b == 0)?;
    // Certains schémas gardent des entités HTML (« Atelier Ryza 3: … &amp; the Secret Key »)
    let name = String::from_utf8_lossy(&buf[i..i + len])
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    Some(name).filter(|s| !s.is_empty())
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
    /// Barre de progression : (stat suivie, min, max)
    progress: Option<(StatDef, f64, f64)>,
    achievement: Achievement,
}

/// Statistique du jeu (hors succès) : clé dans le fichier du compte, type, valeur par défaut.
#[derive(Debug, Clone, Copy)]
struct StatDef {
    id: u32,
    float: bool,
    default: f64,
}

/// Statistiques du jeu par nom (`STAT_BULLDOZER`…).
fn stat_defs(game: &Kv) -> HashMap<String, StatDef> {
    let mut out = HashMap::new();
    for (key, stat) in game.get("stats").map(Kv::children).unwrap_or_default() {
        let (Ok(id), Some(name)) = (key.parse::<u32>(), stat.get("name").and_then(Kv::as_str)) else { continue };
        let float = match stat.get("type") {
            Some(Kv::Str(s)) => matches!(s.trim().to_ascii_uppercase().as_str(), "FLOAT" | "AVGRATE" | "2" | "3"),
            Some(v) => matches!(v.as_i64(), Some(2 | 3)),
            None => false,
        };
        let default = stat.get("default").and_then(Kv::as_f64).unwrap_or(0.0);
        out.insert(name.to_string(), StatDef { id, float, default });
    }
    out
}

/// Progression déclarée sur un succès, reliée à sa statistique.
fn progress_def(bit: &Kv, stats: &HashMap<String, StatDef>) -> Option<(StatDef, f64, f64)> {
    let p = bit.get("progress")?;
    let value = p.get("value")?;
    if !value.get("operation").and_then(Kv::as_str).is_some_and(|o| o.eq_ignore_ascii_case("statvalue")) {
        return None;
    }
    let stat = *stats.get(value.get("operand1")?.as_str()?)?;
    let min = p.get("min_val").and_then(Kv::as_f64).unwrap_or(0.0);
    let max = p.get("max_val").and_then(Kv::as_f64)?;
    (max > min).then_some((stat, min, max))
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

    let stats = stat_defs(game);
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
                progress: progress_def(b, &stats),
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
                    progress: None,
                },
            });
        }
    }
    (game_name, entries)
}

/// Fichier du compte lu : succès débloqués et valeurs brutes des statistiques.
struct UserStats {
    /// Date d'obtention par (stat, bit), 0 si inconnue
    unlocked: HashMap<(u32, u32), i64>,
    /// `data` de chaque statistique
    values: HashMap<u32, Kv>,
}

impl UserStats {
    /// Valeur actuelle d'une statistique (défaut du schéma si le jeu ne l'a jamais écrite).
    fn value(&self, stat: StatDef) -> f64 {
        match self.values.get(&stat.id) {
            Some(Kv::Float(f)) => *f as f64,
            Some(v) if stat.float => v.as_i64().map_or(stat.default, |bits| f32::from_bits(bits as u32) as f64),
            Some(v) => v.as_i64().map_or(stat.default, |i| i as f64),
            None => stat.default,
        }
    }
}

/// État débloqué par (stat, bit) : `data` est un champ de bits, `AchievementTimes` les dates.
fn parse_progress(root: &Kv) -> UserStats {
    let cache = root.get("cache").unwrap_or(root);
    let mut out = HashMap::new();
    let mut values = HashMap::new();
    for (stat_key, stat) in cache.children() {
        let Ok(stat_id) = stat_key.parse::<u32>() else { continue };
        if let Some(data) = stat.get("data") {
            values.insert(stat_id, data.clone());
        }
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
    UserStats { unlocked: out, values }
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
                let t = p.unlocked.get(&(e.stat_id, e.bit));
                a.unlocked = Some(t.is_some());
                a.unlock_time = t.copied().filter(|&t| t > 0);
                a.progress = e.progress.map(|(stat, min, max)| Progress { value: p.value(stat), min, max });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valeur_des_statistiques() {
        let stats = UserStats {
            unlocked: HashMap::new(),
            // Terraria : stat INT, et stat FLOAT rangée en bits de f32 (10 000,0)
            values: HashMap::from([(6, Kv::Int(3)), (9, Kv::Int(1_176_256_512))]),
        };
        let def = |id, float| StatDef { id, float, default: 0.0 };
        assert_eq!(stats.value(def(6, false)), 3.0);
        assert_eq!(stats.value(def(9, true)), 10_000.0);
        // Jamais écrite par le jeu : valeur par défaut du schéma
        assert_eq!(stats.value(StatDef { id: 42, float: false, default: 5.0 }), 5.0);
    }
}
