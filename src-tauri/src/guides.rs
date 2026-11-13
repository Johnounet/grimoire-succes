//! Export / import des guides d'un jeu : un fichier JSON unique, images incluses (base64).
//!
//! Le fichier contient le guide, jamais qui l'a fait ni où il en est : pas de progression Steam, de coches,
//! de compte, de temps de jeu ni de réglages. Les succès y sont repérés par leur ID technique (à défaut,
//! leur nom), pour que le guide marche sur un autre PC. L'état des compteurs, listes et chronos et le statut
//! des cartes ne sont gardés que sur demande (`keep_state`, pour une sauvegarde) ; sinon le guide est vierge.
//!
//! L'import fusionne avec le jeu (créé depuis Steam s'il manque) : les blocs s'ajoutent après ceux déjà
//! présents, et ce qui existe déjà à l'identique n'est pas recréé (importer deux fois le même fichier ne
//! double rien). Les blocs d'un succès introuvable dans ce jeu arrivent dans une carte libre à son nom.

use crate::blocks::store_image;
use crate::db::{self, GameSummary, GridPos, Owner, WorkspaceItem};
use crate::library::AppState;
use base64::{engine::general_purpose::STANDARD, Engine};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

pub const FORMAT: &str = "chasse-succes-guide";
pub const VERSION: u32 = 1;
const MAX_FILE_BYTES: u64 = 300 * 1024 * 1024;

/// Ce qui porte un bloc, sous la forme de la base : (succès, carte libre).
type OwnerKey = (Option<i64>, Option<i64>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideFile {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub app_version: String,
    #[serde(default)]
    pub exported_at: i64,
    pub game: GuideGame,
    /// État des compteurs, listes, chronos et statut des cartes gardés (sauvegarde)
    #[serde(default)]
    pub keeps_state: bool,
    /// Étiquettes utilisées par le guide
    #[serde(default)]
    pub tags: Vec<GuideTag>,
    #[serde(default)]
    pub cards: Vec<GuideCard>,
    /// Images des blocs : nom repris dans `data.file` → contenu en base64
    #[serde(default)]
    pub images: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideGame {
    pub appid: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GuideTag {
    pub name: String,
    pub color: String,
}

/// Carte d'un succès (`achievement`) ou carte libre (`title`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuideCard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub achievement: Option<GuideAchievement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Place dans l'espace de travail ; absente pour un succès qui a des blocs sans être dans l'espace
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<GuidePlace>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub blocks: Vec<GuideBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideAchievement {
    #[serde(default)]
    pub api_name: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuidePlace {
    pub column: String,
    pub collapsed: bool,
    pub grid: Option<GridPos>,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GuideBlock {
    pub kind: String,
    pub data: Value,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// ---------- Export ----------

/// Guide vierge : compteurs à zéro, listes décochées, chronos sans temps ni record.
fn reset_state(kind: &str, data: &mut Value) {
    match kind {
        "counter" => data["value"] = Value::from(0),
        "list" => {
            if let Some(items) = data.get_mut("items").and_then(Value::as_array_mut) {
                for it in items {
                    it["checked"] = Value::Bool(false);
                }
            }
        }
        "timer" => {
            data["elapsed_ms"] = Value::from(0);
            data["running_since"] = Value::Null;
            data["best_ms"] = Value::Null;
        }
        _ => {}
    }
}

pub fn export(conn: &Connection, images_dir: &Path, appid: u32, keep_state: bool) -> Result<GuideFile, String> {
    let game = db::get_game(conn, appid).map_err(err)?.ok_or("jeu introuvable")?;
    let achievements = db::list_achievements(conn, appid).map_err(err)?;
    let by_id: HashMap<i64, &db::AchievementRow> = achievements.iter().map(|a| (a.id, a)).collect();
    let free_titles: HashMap<i64, String> =
        db::list_free_cards(conn, appid).map_err(err)?.into_iter().map(|f| (f.id, f.title)).collect();
    let all_tags: HashMap<i64, db::Tag> = db::list_tags(conn).map_err(err)?.into_iter().map(|t| (t.id, t)).collect();
    let mut tags_of: HashMap<i64, Vec<i64>> = HashMap::new();
    for t in db::list_achievement_tags(conn, appid).map_err(err)? {
        tags_of.entry(t.achievement_id).or_default().push(t.tag_id);
    }
    let mut blocks_of: HashMap<OwnerKey, Vec<db::Block>> = HashMap::new();
    for b in db::list_blocks(conn, appid).map_err(err)? {
        blocks_of.entry((b.achievement_id, b.card_id)).or_default().push(b);
    }

    // Cartes de l'espace de travail, puis succès hors de l'espace qui ont pourtant des blocs ou des étiquettes
    let workspace = db::list_workspace(conn, appid).map_err(err)?;
    let placed: HashSet<i64> = workspace.iter().filter_map(|it| it.achievement_id).collect();
    let extras: Vec<i64> = achievements
        .iter()
        .filter(|a| !placed.contains(&a.id) && (blocks_of.contains_key(&(Some(a.id), None)) || tags_of.contains_key(&a.id)))
        .map(|a| a.id)
        .collect();

    let mut images = BTreeMap::new();
    let mut used_tags: Vec<i64> = Vec::new();
    let mut card = |achievement_id: Option<i64>, card_id: Option<i64>, place: Option<GuidePlace>| -> GuideCard {
        let blocks = blocks_of
            .remove(&(achievement_id, card_id))
            .unwrap_or_default()
            .into_iter()
            .map(|b| {
                let mut data = b.data;
                if !keep_state {
                    reset_state(&b.kind, &mut data);
                }
                if b.kind == "image" {
                    if let Some(file) = data.get("file").and_then(Value::as_str) {
                        if let Ok(bytes) = std::fs::read(images_dir.join(file)) {
                            images.insert(file.to_string(), STANDARD.encode(bytes));
                        }
                    }
                }
                GuideBlock { kind: b.kind, data }
            })
            .collect();
        let tags = achievement_id
            .and_then(|id| tags_of.get(&id))
            .map(|ids| {
                ids.iter()
                    .filter_map(|t| all_tags.get(t))
                    .map(|t| {
                        if !used_tags.contains(&t.id) {
                            used_tags.push(t.id);
                        }
                        t.name.clone()
                    })
                    .collect()
            })
            .unwrap_or_default();
        GuideCard {
            achievement: achievement_id
                .and_then(|id| by_id.get(&id))
                .map(|a| GuideAchievement { api_name: a.api_name.clone(), name: a.name.clone() }),
            title: card_id.map(|id| free_titles.get(&id).cloned().unwrap_or_default()),
            workspace: place,
            tags,
            blocks,
        }
    };

    let mut cards = Vec::new();
    for it in workspace {
        let place = GuidePlace {
            column: if keep_state { it.column } else { "todo".into() },
            collapsed: it.collapsed,
            grid: it.grid,
            pinned: keep_state && it.pinned,
        };
        cards.push(card(it.achievement_id, it.card_id, Some(place)));
    }
    for id in extras {
        cards.push(card(Some(id), None, None));
    }

    let tags = used_tags
        .iter()
        .filter_map(|id| all_tags.get(id))
        .map(|t| GuideTag { name: t.name.clone(), color: t.color.clone() })
        .collect();
    Ok(GuideFile {
        format: FORMAT.into(),
        version: VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        exported_at: now(),
        game: GuideGame { appid, name: game.name },
        keeps_state: keep_state,
        tags,
        cards,
        images,
    })
}

// ---------- Import ----------

/// Bilan d'un import, affiché au joueur.
#[derive(Debug, Clone, Serialize)]
pub struct ImportReport {
    pub game: GameSummary,
    /// Blocs ajoutés
    pub blocks: usize,
    /// Cartes libres créées
    pub free_cards: usize,
    /// Cartes ajoutées à l'espace de travail
    pub placed: usize,
    /// Éléments déjà présents à l'identique, non recréés
    pub duplicates: usize,
    /// Succès du guide introuvables dans ce jeu (leurs blocs sont dans une carte libre)
    pub unmatched: Vec<String>,
}

/// Format et version lisibles par cette version du logiciel.
pub fn check(guide: &GuideFile) -> Result<(), String> {
    if guide.format != FORMAT {
        return Err("Ce fichier n'est pas un guide de Chasse aux succès.".into());
    }
    if guide.version > VERSION {
        return Err("Ce guide vient d'une version plus récente du logiciel : mettez-le à jour pour l'importer.".into());
    }
    Ok(())
}

/// Bloc comparé pour repérer les doublons : une image l'est par sa légende (son fichier change à chaque import).
fn comparable(kind: &str, data: &Value) -> (String, Value) {
    let mut d = data.clone();
    if kind == "image" {
        if let Some(o) = d.as_object_mut() {
            o.remove("file");
        }
    }
    (kind.to_string(), d)
}

pub fn import(conn: &mut Connection, images_dir: &Path, guide: &GuideFile) -> Result<ImportReport, String> {
    check(guide)?;
    let appid = guide.game.appid;
    if db::get_game(conn, appid).map_err(err)?.is_none() {
        return Err("jeu absent de la bibliothèque".into());
    }

    // Succès du jeu : par ID technique, sinon par nom
    let achievements = db::list_achievements(conn, appid).map_err(err)?;
    let by_api: HashMap<&str, i64> =
        achievements.iter().filter_map(|a| a.api_name.as_deref().map(|n| (n, a.id))).collect();
    let by_name: HashMap<&str, i64> = achievements.iter().map(|a| (a.name.as_str(), a.id)).collect();

    let mut existing: HashMap<OwnerKey, Vec<(String, Value)>> = HashMap::new();
    for b in db::list_blocks(conn, appid).map_err(err)? {
        existing.entry((b.achievement_id, b.card_id)).or_default().push(comparable(&b.kind, &b.data));
    }
    let free_cards = db::list_free_cards(conn, appid).map_err(err)?;

    let mut workspace = db::list_workspace(conn, appid).map_err(err)?;
    let keep_grid = workspace.is_empty();
    let mut present: HashSet<i64> = workspace.iter().filter_map(|it| it.achievement_id).collect();

    let mut tag_ids: HashMap<String, i64> =
        db::list_tags(conn).map_err(err)?.into_iter().map(|t| (t.name.to_lowercase(), t.id)).collect();
    let colors: HashMap<String, &str> = guide.tags.iter().map(|t| (t.name.to_lowercase(), t.color.as_str())).collect();

    // Images : chaque fichier du guide n'est enregistré qu'une fois, même s'il sert à plusieurs blocs
    let mut stored: HashMap<String, Option<String>> = HashMap::new();
    let mut image = |name: &str| -> Option<String> {
        stored
            .entry(name.to_string())
            .or_insert_with(|| {
                let bytes = STANDARD.decode(guide.images.get(name)?).ok()?;
                store_image(images_dir, &bytes).ok()
            })
            .clone()
    };

    let mut report = ImportReport {
        game: db::get_game(conn, appid).map_err(err)?.ok_or("jeu introuvable")?,
        blocks: 0,
        free_cards: 0,
        placed: 0,
        duplicates: 0,
        unmatched: Vec::new(),
    };

    let place = |p: &GuidePlace, achievement_id: Option<i64>, card_id: Option<i64>| WorkspaceItem {
        achievement_id,
        card_id,
        column: p.column.clone(),
        collapsed: p.collapsed,
        grid: if keep_grid { p.grid } else { None },
        pinned: p.pinned,
    };
    let default_place = GuidePlace { column: "todo".into(), collapsed: false, grid: None, pinned: false };

    for c in &guide.cards {
        let matched = c.achievement.as_ref().and_then(|a| {
            a.api_name
                .as_deref()
                .and_then(|n| by_api.get(n))
                .or_else(|| by_name.get(a.name.as_str()))
                .copied()
        });

        let owner = match (&c.achievement, matched) {
            (Some(_), Some(id)) => Owner::Achievement(id),
            (achievement, _) => {
                // Carte libre, ou succès introuvable : carte libre à son nom
                let title = match achievement {
                    Some(a) => {
                        report.unmatched.push(a.name.clone());
                        if c.blocks.is_empty() {
                            continue;
                        }
                        format!("Succès introuvable : {}", a.name)
                    }
                    None => c.title.clone().unwrap_or_default(),
                };
                let wanted: Vec<(String, Value)> = c.blocks.iter().map(|b| comparable(&b.kind, &b.data)).collect();
                let same = free_cards
                    .iter()
                    .any(|f| f.title == title && existing.get(&(None, Some(f.id))).map(Vec::as_slice).unwrap_or(&[]) == wanted);
                if same {
                    report.duplicates += 1;
                    continue;
                }
                let f = db::create_free_card(conn, appid, &title).map_err(err)?;
                report.free_cards += 1;
                workspace.push(place(c.workspace.as_ref().unwrap_or(&default_place), None, Some(f.id)));
                report.placed += 1;
                Owner::Card(f.id)
            }
        };

        let key = match owner {
            Owner::Achievement(id) => (Some(id), None),
            Owner::Card(id) => (None, Some(id)),
        };
        for b in &c.blocks {
            let cmp = comparable(&b.kind, &b.data);
            if existing.get(&key).is_some_and(|list| list.contains(&cmp)) {
                report.duplicates += 1;
                continue;
            }
            let mut data = b.data.clone();
            if b.kind == "image" {
                let file = data.get("file").and_then(Value::as_str).and_then(&mut image);
                data["file"] = file.map(Value::from).unwrap_or(Value::Null);
            }
            db::create_block(conn, owner, &b.kind, &data, None).map_err(err)?;
            existing.entry(key).or_default().push(cmp);
            report.blocks += 1;
        }

        if let Owner::Achievement(id) = owner {
            for name in &c.tags {
                let lower = name.to_lowercase();
                let tag = match tag_ids.get(&lower) {
                    Some(&t) => t,
                    None => {
                        let color = colors.get(&lower).copied().unwrap_or("#4f7cff");
                        let t = db::create_tag(conn, name, color).or_else(|_| db::create_tag(conn, name, "#4f7cff"))?;
                        tag_ids.insert(lower, t.id);
                        t.id
                    }
                };
                db::set_achievement_tag(conn, id, tag, true).map_err(err)?;
            }
            if let Some(p) = &c.workspace {
                if present.insert(id) {
                    workspace.push(place(p, Some(id), None));
                    report.placed += 1;
                }
            }
        }
    }

    db::save_workspace(conn, appid, &workspace).map_err(err)?;
    Ok(report)
}

// ---------- Fichiers et commandes ----------

/// Nom de fichier proposé : le nom du jeu sans les caractères interdits par Windows.
fn file_name(game: &str) -> String {
    let clean: String = game.chars().map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { ' ' } else { c }).collect();
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("{} - guide.json", if clean.is_empty() { "jeu" } else { &clean })
}

pub fn read_guide(path: &Path) -> Result<GuideFile, String> {
    let size = std::fs::metadata(path).map_err(err)?.len();
    if size > MAX_FILE_BYTES {
        return Err("Fichier trop lourd pour un guide.".into());
    }
    let bytes = std::fs::read(path).map_err(err)?;
    let guide: GuideFile =
        serde_json::from_slice(&bytes).map_err(|_| "Ce fichier n'est pas un guide de Chasse aux succès.".to_string())?;
    check(&guide)?;
    Ok(guide)
}

/// Exporte les guides du jeu dans un fichier choisi par le joueur. Renvoie le chemin, ou `None` si annulé.
#[tauri::command]
pub async fn guide_export(
    app: AppHandle,
    state: State<'_, AppState>,
    appid: u32,
    keep_state: bool,
) -> Result<Option<String>, String> {
    let guide = export(&state.conn(), &state.images_dir, appid, keep_state)?;
    let Some(path) = app
        .dialog()
        .file()
        .set_title("Exporter le guide")
        .add_filter("Guide Chasse aux succès", &["json"])
        .set_file_name(file_name(&guide.game.name))
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = path.into_path().map_err(err)?;
    std::fs::write(&path, serde_json::to_vec_pretty(&guide).map_err(err)?).map_err(err)?;
    Ok(Some(path.display().to_string()))
}

/// Importe un guide choisi par le joueur. Le jeu est d'abord ajouté depuis Steam s'il manque.
/// Renvoie le bilan, ou `None` si annulé.
#[tauri::command]
pub async fn guide_import(app: AppHandle, state: State<'_, AppState>) -> Result<Option<ImportReport>, String> {
    let Some(path) = app
        .dialog()
        .file()
        .set_title("Importer un guide")
        .add_filter("Guide Chasse aux succès", &["json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let guide = read_guide(&path.into_path().map_err(err)?)?;
    let appid = guide.game.appid;
    let known = db::game_exists(&state.conn(), appid).map_err(err)?;
    if !known {
        crate::library::library_import(state.clone(), appid)
            .await
            .map_err(|e| format!("Impossible d'ajouter « {} » depuis Steam : {e}", guide.game.name))?;
    }
    import(&mut state.conn(), &state.images_dir, &guide).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::steam::model::{Achievement, GameAchievements, Source};
    use serde_json::json;

    fn ach(api: &str) -> Achievement {
        Achievement {
            api_name: Some(api.into()),
            name: api.to_lowercase(),
            description: String::new(),
            hidden: false,
            icon_url: None,
            icon_gray_url: None,
            rarity: None,
            unlocked: Some(api == "A"),
            unlock_time: None,
            dlc_appid: None,
        }
    }

    fn setup(apis: &[&str]) -> Connection {
        let mut conn = db::open_in_memory().unwrap();
        let data = GameAchievements {
            appid: 620,
            game_name: None,
            source: Source::LocalCache,
            language: "french".into(),
            achievements: apis.iter().map(|a| ach(a)).collect(),
        };
        db::save_game(&mut conn, &data, "Portal 2", None).unwrap();
        conn
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("chasse-succes-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("blocks")).unwrap();
        dir
    }

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfaux";

    #[test]
    fn export_puis_import_ailleurs() {
        let dir = temp_dir("a");
        let mut conn = setup(&["A", "B", "C"]);
        let ids: Vec<i64> = db::list_achievements(&conn, 620).unwrap().iter().map(|a| a.id).collect();
        std::fs::write(dir.join("blocks/x.png"), PNG).unwrap();

        // Espace : A (en cours, épinglé), une carte libre ; C a un bloc sans être dans l'espace
        db::create_block(&mut conn, Owner::Achievement(ids[0]), "counter", &json!({"label": "cubes", "value": 7, "goal": 12, "step": 1}), None).unwrap();
        db::create_block(&mut conn, Owner::Achievement(ids[0]), "image", &json!({"file": "blocks/x.png", "caption": "carte"}), None).unwrap();
        db::create_block(&mut conn, Owner::Achievement(ids[2]), "list", &json!({"items": [{"text": "radio", "checked": true}]}), None).unwrap();
        let free = db::create_free_card(&conn, 620, "Route").unwrap();
        db::create_block(&mut conn, Owner::Card(free.id), "timer", &json!({"label": "", "target_ms": null, "best_ms": 900, "elapsed_ms": 50, "running_since": null}), None).unwrap();
        let grind = db::list_tags(&conn).unwrap().into_iter().find(|t| t.name == "grind").unwrap();
        db::set_achievement_tag(&conn, ids[0], grind.id, true).unwrap();
        let item = |a: Option<i64>, c: Option<i64>| WorkspaceItem {
            achievement_id: a,
            card_id: c,
            column: "doing".into(),
            collapsed: false,
            grid: Some(GridPos { x: 0, y: 0, w: 4, h: 8 }),
            pinned: true,
        };
        db::save_workspace(&mut conn, 620, &[item(Some(ids[0]), None), item(None, Some(free.id))]).unwrap();

        // Guide vierge : ni progression ni état, mais tout le contenu
        let guide = export(&conn, &dir, 620, false).unwrap();
        let text = serde_json::to_string(&guide).unwrap();
        assert!(!text.contains("\"unlocked\"") && !text.contains("\"pinned\":true") && !text.contains("doing"));
        assert_eq!(guide.cards.len(), 3);
        assert_eq!(guide.cards[0].blocks[0].data["value"], 0);
        assert_eq!(guide.cards[0].tags, vec!["grind"]);
        assert_eq!(guide.cards[1].title.as_deref(), Some("Route"));
        assert_eq!(guide.cards[1].blocks[0].data["best_ms"], Value::Null);
        assert!(guide.cards[2].workspace.is_none() && guide.cards[2].blocks[0].data["items"][0]["checked"] == false);
        assert_eq!(guide.tags, vec![GuideTag { name: "grind".into(), color: grind.color.clone() }]);
        assert!(guide.images.contains_key("blocks/x.png"));
        // Sauvegarde : l'état est gardé
        let backup = export(&conn, &dir, 620, true).unwrap();
        assert_eq!(backup.cards[0].blocks[0].data["value"], 7);
        assert!(backup.cards[0].workspace.as_ref().unwrap().pinned);

        // Import dans une autre installation : B absent du jeu, nouvelle étiquette créée, image recopiée
        let dir2 = temp_dir("b");
        let mut other = setup(&["A", "C"]);
        db::delete_tag(&other, db::list_tags(&other).unwrap().into_iter().find(|t| t.name == "grind").unwrap().id).unwrap();
        let mut g = guide.clone();
        g.cards.push(GuideCard {
            achievement: Some(GuideAchievement { api_name: Some("B".into()), name: "b".into() }),
            blocks: vec![GuideBlock { kind: "text".into(), data: json!({"markdown": "note"}) }],
            ..Default::default()
        });
        let r = import(&mut other, &dir2, &g).unwrap();
        assert_eq!((r.blocks, r.free_cards, r.duplicates, r.unmatched.clone()), (5, 2, 0, vec!["b".to_string()]));
        let ws = db::list_workspace(&other, 620).unwrap();
        assert_eq!(ws.len(), 3, "A, la carte libre et la carte du succès introuvable");
        assert_eq!(ws[0].grid, Some(GridPos { x: 0, y: 0, w: 4, h: 8 }), "espace vide : la disposition est reprise");
        assert!(db::list_free_cards(&other, 620).unwrap().iter().any(|f| f.title == "Succès introuvable : b"));
        assert_eq!(db::list_achievement_tags(&other, 620).unwrap().len(), 1);
        let img = db::list_blocks(&other, 620).unwrap().into_iter().find(|b| b.kind == "image").unwrap();
        let file = img.data["file"].as_str().unwrap().to_string();
        assert_eq!(std::fs::read(dir2.join(&file)).unwrap(), PNG);
        let ach_a = db::list_achievements(&other, 620).unwrap().into_iter().find(|a| a.api_name.as_deref() == Some("A")).unwrap();
        assert!(ach_a.unlocked, "la progression locale n'est pas touchée");

        // Réimporter le même fichier ne double rien
        let again = import(&mut other, &dir2, &g).unwrap();
        assert_eq!((again.blocks, again.free_cards, again.placed), (0, 0, 0));
        assert_eq!(db::list_blocks(&other, 620).unwrap().len(), 5);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir2);
    }

    #[test]
    fn refuse_un_autre_fichier() {
        let mut g = GuideFile {
            format: "autre".into(),
            version: 1,
            app_version: String::new(),
            exported_at: 0,
            game: GuideGame { appid: 620, name: "P".into() },
            keeps_state: false,
            tags: vec![],
            cards: vec![],
            images: BTreeMap::new(),
        };
        assert!(check(&g).is_err());
        g.format = FORMAT.into();
        g.version = VERSION + 1;
        assert!(check(&g).is_err());
        assert_eq!(file_name("Portal 2: \"édition\" <x>"), "Portal 2 édition x - guide.json");
        assert_eq!(file_name("Portal 2"), "Portal 2 - guide.json");
    }
}
