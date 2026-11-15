//! Export / import des guides d'un jeu : un fichier JSON unique, images incluses (base64).
//!
//! Le fichier contient le guide, jamais qui l'a fait ni où il en est : pas de progression Steam, de coches,
//! de compte, de temps de jeu ni de réglages. Les succès y sont repérés par leur ID technique (à défaut,
//! leur nom), pour que le guide marche sur un autre PC. L'état des compteurs, listes, chronos et
//! l'épingle au mode compact ne sont gardés que sur demande (`keep_state`, pour une sauvegarde) ; sinon le guide est vierge.
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

pub const FORMAT: &str = "grimoire-guide";
/// Format des guides exportés avant que le logiciel s'appelle Grimoire (« Chasse aux succès ») : toujours lisibles.
const LEGACY_FORMATS: &[&str] = &["chasse-succes-guide"];
pub const VERSION: u32 = 1;
const MAX_FILE_BYTES: u64 = 300 * 1024 * 1024;
/// Couleur d'une étiquette du guide sans couleur valide.
const DEFAULT_TAG_COLOR: &str = "#4f7cff";

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
    /// État des compteurs, listes, chronos et épingles des cartes gardés (sauvegarde)
    #[serde(default)]
    pub keeps_state: bool,
    /// Étiquettes utilisées par le guide
    #[serde(default)]
    pub tags: Vec<GuideTag>,
    /// Sections de l'espace de travail, dans l'ordre (les cartes y renvoient par leur rang)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<GuideSection>,
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
    /// Carte libre cochée : gardé seulement avec l'état (sauvegarde), comme les coches des succès ne le sont jamais
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideAchievement {
    #[serde(default)]
    pub api_name: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GuideSection {
    pub title: String,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuidePlace {
    pub collapsed: bool,
    pub grid: Option<GridPos>,
    pub pinned: bool,
    /// Rang de la section dans `GuideFile::sections` ; absent : hors section
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<usize>,
    /// Succès dans la carte duquel celui-ci est rattaché (un seul niveau)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<GuideAchievement>,
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
    let free_cards: HashMap<i64, db::FreeCard> =
        db::list_free_cards(conn, appid).map_err(err)?.into_iter().map(|f| (f.id, f)).collect();
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
    let sections = db::list_sections(conn, appid).map_err(err)?;
    let section_rank: HashMap<i64, usize> = sections.iter().enumerate().map(|(i, s)| (s.id, i)).collect();
    let placed: HashSet<i64> = workspace.iter().filter_map(|it| it.achievement_id).collect();
    let extras: Vec<i64> = achievements
        .iter()
        .filter(|a| !placed.contains(&a.id) && (blocks_of.contains_key(&(Some(a.id), None)) || tags_of.contains_key(&a.id)))
        .map(|a| a.id)
        .collect();

    let with_blocks: HashSet<i64> = blocks_of.keys().filter_map(|&(a, c)| if c.is_none() { a } else { None }).collect();

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
            title: card_id.map(|id| free_cards.get(&id).map(|f| f.title.clone()).unwrap_or_default()),
            done: keep_state && card_id.and_then(|id| free_cards.get(&id)).is_some_and(|f| f.done),
            workspace: place,
            tags,
            blocks,
        }
    };

    // Succès qui en contiennent d'autres : gardés dans le guide, pour que le rattachement s'y retrouve
    let parents: HashSet<i64> = workspace.iter().filter_map(|it| it.parent).collect();
    let mut cards = Vec::new();
    for it in workspace {
        // Vue unique : tous les succès sont dans l'espace. Seuls ceux qui portent quelque chose de
        // personnel (blocs, étiquettes, section, épingle) vont dans le guide, pas les 200 succès du jeu.
        if let Some(id) = it.achievement_id {
            let personal = with_blocks.contains(&id)
                || tags_of.contains_key(&id)
                || it.section.is_some()
                || it.parent.is_some()
                || parents.contains(&id)
                || (keep_state && it.pinned);
            if !personal {
                continue;
            }
        }
        let place = GuidePlace {
            collapsed: it.collapsed,
            grid: it.grid,
            pinned: keep_state && it.pinned,
            section: it.section.and_then(|id| section_rank.get(&id).copied()),
            parent: it
                .parent
                .and_then(|id| by_id.get(&id))
                .map(|a| GuideAchievement { api_name: a.api_name.clone(), name: a.name.clone() }),
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
        sections: sections.into_iter().map(|s| GuideSection { title: s.title, collapsed: s.collapsed, color: s.color }).collect(),
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
    /// Étiquettes du guide refusées (nom vide ou trop long) : ignorées, le reste est importé
    pub ignored_tags: Vec<String>,
}

/// Format et version lisibles par cette version du logiciel.
pub fn check(guide: &GuideFile) -> Result<(), String> {
    if guide.format != FORMAT && !LEGACY_FORMATS.contains(&guide.format.as_str()) {
        return Err("Ce fichier n'est pas un guide de Grimoire.".into());
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

/// Import « tout ou rien » : en cas d'erreur, la base revient à son état d'avant et les images recopiées sont effacées.
pub fn import(conn: &mut Connection, images_dir: &Path, guide: &GuideFile) -> Result<ImportReport, String> {
    check(guide)?;
    conn.execute_batch("SAVEPOINT guide_import").map_err(err)?;
    let mut copied: Vec<String> = Vec::new();
    // Une validation finale ratée (disque plein…) passe aussi par l'annulation : le savepoint ne reste jamais ouvert
    let result = import_all(conn, images_dir, guide, &mut copied)
        .and_then(|report| conn.execute_batch("RELEASE guide_import").map(|_| report).map_err(err));
    match result {
        Ok(report) => Ok(report),
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK TO guide_import; RELEASE guide_import");
            for file in copied {
                let _ = std::fs::remove_file(images_dir.join(file));
            }
            Err(e)
        }
    }
}

fn import_all(
    conn: &mut Connection,
    images_dir: &Path,
    guide: &GuideFile,
    copied: &mut Vec<String>,
) -> Result<ImportReport, String> {
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
    let colors: HashMap<String, &str> =
        guide.tags.iter().map(|t| (t.name.trim().to_lowercase(), t.color.as_str())).collect();

    // Images : chaque fichier du guide n'est enregistré qu'une fois, même s'il sert à plusieurs blocs
    let mut stored: HashMap<String, Option<String>> = HashMap::new();
    let mut image = |name: &str| -> Option<String> {
        stored
            .entry(name.to_string())
            .or_insert_with(|| {
                let bytes = STANDARD.decode(guide.images.get(name)?).ok()?;
                let file = store_image(images_dir, &bytes).ok()?;
                copied.push(file.clone());
                Some(file)
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
        ignored_tags: Vec::new(),
    };

    // Sections du guide : reprises par leur nom si le jeu en a déjà une pareille, créées sinon
    let mut local_sections = db::list_sections(conn, appid).map_err(err)?;
    let mut section_ids: Vec<i64> = Vec::new();
    for gs in &guide.sections {
        let id = match local_sections.iter().find(|s| s.title == gs.title.trim()) {
            Some(s) => s.id,
            None => {
                let s = db::create_section(conn, appid, &gs.title, &gs.color).map_err(err)?;
                local_sections.push(db::Section { collapsed: gs.collapsed, ..s.clone() });
                s.id
            }
        };
        section_ids.push(id);
    }
    db::save_sections(conn, appid, &local_sections).map_err(err)?;

    let place = |p: &GuidePlace, achievement_id: Option<i64>, card_id: Option<i64>| WorkspaceItem {
        achievement_id,
        card_id,
        collapsed: p.collapsed,
        grid: if keep_grid { p.grid } else { None },
        pinned: p.pinned,
        section: p.section.and_then(|i| section_ids.get(i).copied()),
        parent: None,
    };
    let default_place = GuidePlace { collapsed: false, grid: None, pinned: false, section: None, parent: None };
    let find = |a: &GuideAchievement| -> Option<i64> {
        a.api_name.as_deref().and_then(|n| by_api.get(n)).or_else(|| by_name.get(a.name.as_str())).copied()
    };
    // Rattachements du guide (succès, succès parent), appliqués une fois toutes les cartes placées
    let mut links: Vec<(i64, i64)> = Vec::new();

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
                if c.done {
                    db::set_free_card_done(conn, f.id, true).map_err(err)?;
                }
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
        // Chaque bloc déjà présent n'absorbe qu'un seul bloc identique du guide (il est retiré une fois reconnu),
        // et les blocs créés ne s'ajoutent pas à la liste : deux images sans légende d'une même carte restent deux.
        for b in &c.blocks {
            let cmp = comparable(&b.kind, &b.data);
            if let Some(list) = existing.get_mut(&key) {
                if let Some(i) = list.iter().position(|x| *x == cmp) {
                    list.swap_remove(i);
                    report.duplicates += 1;
                    continue;
                }
            }
            let mut data = b.data.clone();
            if b.kind == "image" {
                let file = data.get("file").and_then(Value::as_str).and_then(&mut image);
                data["file"] = file.map(Value::from).unwrap_or(Value::Null);
            }
            db::create_block(conn, owner, &b.kind, &data, None).map_err(err)?;
            report.blocks += 1;
        }

        if let Owner::Achievement(id) = owner {
            for name in &c.tags {
                let name = name.trim();
                let lower = name.to_lowercase();
                let tag = match tag_ids.get(&lower) {
                    Some(&t) => t,
                    None => {
                        // Nom invalide (vide ou trop long) : étiquette ignorée et signalée, sans bloquer l'import.
                        // Toute autre erreur (panne de la base…) remonte et annule l'import.
                        if db::check_tag(name, DEFAULT_TAG_COLOR).is_err() {
                            if !report.ignored_tags.iter().any(|n| n == name) {
                                report.ignored_tags.push(name.to_string());
                            }
                            continue;
                        }
                        let color = colors.get(&lower).copied().unwrap_or(DEFAULT_TAG_COLOR);
                        let color = if db::check_tag(name, color).is_ok() { color } else { DEFAULT_TAG_COLOR };
                        let t = db::create_tag(conn, name, color)?;
                        tag_ids.insert(lower, t.id);
                        t.id
                    }
                };
                db::set_achievement_tag(conn, id, tag, true).map_err(err)?;
            }
            if let Some(parent) = c.workspace.as_ref().and_then(|p| p.parent.as_ref()).and_then(find) {
                links.push((id, parent));
            }
            if let Some(p) = &c.workspace {
                if present.insert(id) {
                    workspace.push(place(p, Some(id), None));
                    report.placed += 1;
                } else if let Some(section) = place(p, Some(id), None).section {
                    // Vue unique : le succès a déjà sa place ; la section du guide l'y range s'il n'en a pas
                    if let Some(i) = workspace.iter().position(|it| it.achievement_id == Some(id) && it.section.is_none()) {
                        let mut it = workspace.remove(i);
                        it.section = Some(section);
                        workspace.push(it);
                        report.placed += 1;
                    }
                }
            }
        }
    }

    // Rattachements : seulement pour un succès encore libre ici (ni rattaché, ni parent d'autres succès),
    // vers un parent présent et lui-même libre. `save_workspace` vérifie encore le « un seul niveau ».
    for (child, parent) in links {
        let has_children = |id: i64| workspace.iter().any(|it| it.parent == Some(id));
        let free_here = |id: i64| workspace.iter().any(|it| it.achievement_id == Some(id) && it.parent.is_none());
        if child == parent || has_children(child) || !free_here(child) || !free_here(parent) {
            continue;
        }
        if let Some(it) = workspace.iter_mut().find(|it| it.achievement_id == Some(child)) {
            it.parent = Some(parent);
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
        serde_json::from_slice(&bytes).map_err(|_| "Ce fichier n'est pas un guide de Grimoire.".to_string())?;
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
        .add_filter("Guide Grimoire", &["json"])
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
        .add_filter("Guide Grimoire", &["json"])
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
            progress: None,
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
        let dir = std::env::temp_dir().join(format!("grimoire-test-{name}-{}", std::process::id()));
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
            collapsed: false,
            grid: Some(GridPos { x: 0, y: 0, w: 4, h: 8 }),
            pinned: true,
            section: None,
            parent: None,
        };
        let phase = db::create_section(&conn, 620, "Phase 1", "ambre").unwrap();
        let in_phase = WorkspaceItem { section: Some(phase.id), ..item(Some(ids[0]), None) };
        db::save_workspace(&mut conn, 620, &[in_phase, item(None, Some(free.id))]).unwrap();
        db::set_free_card_done(&conn, free.id, true).unwrap();

        // Guide vierge : ni progression ni état, mais tout le contenu
        let guide = export(&conn, &dir, 620, false).unwrap();
        let text = serde_json::to_string(&guide).unwrap();
        assert!(!text.contains("\"unlocked\"") && !text.contains("\"pinned\":true") && !text.contains("\"done\""));
        assert_eq!(guide.cards.len(), 3);
        assert_eq!(guide.cards[0].blocks[0].data["value"], 0);
        assert_eq!(guide.cards[0].tags, vec!["grind"]);
        assert_eq!(guide.cards[1].title.as_deref(), Some("Route"));
        assert_eq!(guide.cards[1].blocks[0].data["best_ms"], Value::Null);
        assert!(guide.cards[2].workspace.is_none() && guide.cards[2].blocks[0].data["items"][0]["checked"] == false);
        assert_eq!(guide.tags, vec![GuideTag { name: "grind".into(), color: grind.color.clone() }]);
        assert!(guide.images.contains_key("blocks/x.png"));
        assert_eq!(guide.sections, vec![GuideSection { title: "Phase 1".into(), collapsed: false, color: "ambre".into() }]);
        assert_eq!(guide.cards[0].workspace.as_ref().unwrap().section, Some(0));
        assert_eq!(guide.cards[1].workspace.as_ref().unwrap().section, None);
        // Sauvegarde : l'état est gardé
        let backup = export(&conn, &dir, 620, true).unwrap();
        assert_eq!(backup.cards[0].blocks[0].data["value"], 7);
        assert!(backup.cards[0].workspace.as_ref().unwrap().pinned);
        assert!(backup.cards[1].done, "carte libre cochée");
        let mut fresh = setup(&["A"]);
        let dir3 = temp_dir("c");
        import(&mut fresh, &dir3, &backup).unwrap();
        let _ = std::fs::remove_dir_all(&dir3);
        assert!(db::list_free_cards(&fresh, 620).unwrap().iter().any(|f| f.title == "Route" && f.done));

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
        assert!(db::list_free_cards(&other, 620).unwrap().iter().all(|f| !f.done), "guide vierge : rien de coché");
        assert_eq!(db::list_achievement_tags(&other, 620).unwrap().len(), 1);
        let img = db::list_blocks(&other, 620).unwrap().into_iter().find(|b| b.kind == "image").unwrap();
        let file = img.data["file"].as_str().unwrap().to_string();
        assert_eq!(std::fs::read(dir2.join(&file)).unwrap(), PNG);
        let ach_a = db::list_achievements(&other, 620).unwrap().into_iter().find(|a| a.api_name.as_deref() == Some("A")).unwrap();
        assert!(ach_a.unlocked, "la progression locale n'est pas touchée");
        let sections = db::list_sections(&other, 620).unwrap();
        assert_eq!(sections.iter().map(|s| (s.title.as_str(), s.color.as_str())).collect::<Vec<_>>(), vec![("Phase 1", "ambre")]);
        assert_eq!(ws[0].section, Some(sections[0].id), "A retrouve sa section");

        // Réimporter le même fichier ne double rien
        let again = import(&mut other, &dir2, &g).unwrap();
        assert_eq!((again.blocks, again.free_cards, again.placed), (0, 0, 0));
        assert_eq!(db::list_blocks(&other, 620).unwrap().len(), 5);
        assert_eq!(db::list_sections(&other, 620).unwrap().len(), 1, "section reprise, pas recréée");

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir2);
    }

    #[test]
    fn vue_unique_export_leger_et_section_reprise() {
        let dir = temp_dir("vue");
        let mut conn = setup(&["A", "B", "C"]);
        let ids: Vec<i64> = db::list_achievements(&conn, 620).unwrap().iter().map(|a| a.id).collect();
        // Tous les succès sont dans l'espace ; seul B a un bloc, dans une section
        let phase = db::create_section(&conn, 620, "Phase 1", "ambre").unwrap();
        db::create_block(&mut conn, Owner::Achievement(ids[1]), "text", &json!({"markdown": "note"}), None).unwrap();
        let mut ws: Vec<WorkspaceItem> = ids
            .iter()
            .map(|&id| WorkspaceItem { achievement_id: Some(id), card_id: None, collapsed: false, grid: None, pinned: false, section: None, parent: None })
            .collect();
        ws[1].section = Some(phase.id);
        db::save_workspace(&mut conn, 620, &ws).unwrap();
        let guide = export(&conn, &dir, 620, false).unwrap();
        assert_eq!(guide.cards.len(), 1, "les succès sans rien de personnel ne sont pas exportés");

        // Import chez quelqu'un dont l'espace contient déjà tous les succès (hors section) : B rejoint la section
        let mut other = setup(&["A", "B", "C"]);
        db::fill_workspace(&other, 620).unwrap();
        import(&mut other, &dir, &guide).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let b = db::list_achievements(&other, 620).unwrap()[1].id;
        let section = db::list_sections(&other, 620).unwrap()[0].id;
        let ws = db::list_workspace(&other, 620).unwrap();
        assert_eq!(ws.len(), 3);
        assert_eq!(ws.iter().find(|it| it.achievement_id == Some(b)).unwrap().section, Some(section));
    }

    fn guide_vide() -> GuideFile {
        GuideFile {
            format: FORMAT.into(),
            version: VERSION,
            app_version: String::new(),
            exported_at: 0,
            game: GuideGame { appid: 620, name: "Portal 2".into() },
            keeps_state: false,
            tags: vec![],
            sections: vec![],
            cards: vec![],
            images: BTreeMap::new(),
        }
    }

    #[test]
    fn etiquette_fautive_ignoree() {
        let dir = temp_dir("c");
        let mut conn = setup(&["A"]);
        let before = db::list_tags(&conn).unwrap().len();
        let mut g = guide_vide();
        g.sections.push(GuideSection { title: "Phase 1".into(), collapsed: false, color: "ambre".into() });
        let place = GuidePlace { collapsed: false, grid: None, pinned: false, section: Some(0), parent: None };
        g.cards.push(GuideCard {
            achievement: Some(GuideAchievement { api_name: Some("A".into()), name: "a".into() }),
            tags: vec!["Grind ".into(), "x".repeat(41), "  ".into()],
            workspace: Some(place.clone()),
            ..Default::default()
        });
        g.cards.push(GuideCard { title: Some("Route".into()), workspace: Some(place), ..Default::default() });

        let r = import(&mut conn, &dir, &g).unwrap();
        assert_eq!(r.ignored_tags, vec!["x".repeat(41), String::new()]);
        assert_eq!(db::list_tags(&conn).unwrap().len(), before, "« Grind  » retrouve l'étiquette « grind »");
        assert_eq!(db::list_achievement_tags(&conn, 620).unwrap().len(), 1);
        assert_eq!(db::list_workspace(&conn, 620).unwrap().len(), 2, "la disposition est enregistrée");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Des blocs identiques d'une même carte (images sans légende, compteurs remis à 0…) sont tous importés,
    /// et réimporter le fichier n'en double aucun.
    #[test]
    fn blocs_identiques_d_une_carte_tous_importes() {
        let dir = temp_dir("e");
        let mut conn = setup(&["A"]);
        let mut g = guide_vide();
        g.images.insert("blocks/x.png".into(), STANDARD.encode(PNG));
        let image = GuideBlock { kind: "image".into(), data: json!({"file": "blocks/x.png", "caption": ""}) };
        let counter = GuideBlock { kind: "counter".into(), data: json!({"label": "cubes", "value": 0, "goal": 3, "step": 1}) };
        let list = GuideBlock { kind: "list".into(), data: json!({"items": []}) };
        let blocks = vec![image.clone(), image.clone(), counter.clone(), counter, list.clone(), list];
        g.cards.push(GuideCard {
            achievement: Some(GuideAchievement { api_name: Some("A".into()), name: "a".into() }),
            blocks: blocks.clone(),
            ..Default::default()
        });
        g.cards.push(GuideCard { title: Some("Route".into()), blocks, ..Default::default() });

        let r = import(&mut conn, &dir, &g).unwrap();
        assert_eq!((r.blocks, r.duplicates), (12, 0));
        let again = import(&mut conn, &dir, &g).unwrap();
        assert_eq!((again.blocks, again.free_cards, again.duplicates), (0, 0, 7), "6 blocs du succès + la carte libre");
        assert_eq!(db::list_blocks(&conn, 620).unwrap().len(), 12);

        // Une seule image déjà présente : la seconde du guide est ajoutée
        let mut conn2 = setup(&["A"]);
        let a = db::list_achievements(&conn2, 620).unwrap()[0].id;
        db::create_block(&mut conn2, Owner::Achievement(a), "image", &json!({"file": null, "caption": ""}), None).unwrap();
        g.cards.truncate(1);
        g.cards[0].blocks.truncate(2);
        let r2 = import(&mut conn2, &dir, &g).unwrap();
        assert_eq!((r2.blocks, r2.duplicates), (1, 1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rattachement_suit_le_guide() {
        let dir = temp_dir("r");
        let mut conn = setup(&["A", "B", "C"]);
        let ids: Vec<i64> = db::list_achievements(&conn, 620).unwrap().iter().map(|a| a.id).collect();
        let item = |id: i64, parent: Option<i64>| WorkspaceItem {
            achievement_id: Some(id),
            card_id: None,
            collapsed: false,
            grid: None,
            pinned: false,
            section: None,
            parent,
        };
        db::save_workspace(&mut conn, 620, &[item(ids[0], None), item(ids[1], Some(ids[0])), item(ids[2], None)]).unwrap();
        let guide = export(&conn, &dir, 620, false).unwrap();
        // A (parent) et B (rattaché) sont dans le guide, même sans bloc ; C non
        assert_eq!(guide.cards.len(), 2);
        assert_eq!(guide.cards[1].workspace.as_ref().unwrap().parent.as_ref().unwrap().api_name.as_deref(), Some("A"));

        let mut other = setup(&["A", "B", "C"]);
        db::fill_workspace(&other, 620).unwrap();
        import(&mut other, &dir, &guide).unwrap();
        let ws = db::list_workspace(&other, 620).unwrap();
        let b = ws.iter().find(|it| it.achievement_id == Some(ids[1])).unwrap();
        assert_eq!(b.parent, Some(ids[0]), "B retrouve sa place dans A");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_rate_ne_laisse_rien() {
        let dir = temp_dir("d");
        let mut conn = setup(&["A"]);
        let mut g = guide_vide();
        g.sections.push(GuideSection { title: "Phase 1".into(), collapsed: false, color: "ambre".into() });
        g.images.insert("blocks/x.png".into(), STANDARD.encode(PNG));
        g.cards.push(GuideCard {
            title: Some("Route".into()),
            blocks: vec![
                GuideBlock { kind: "image".into(), data: json!({"file": "blocks/x.png", "caption": ""}) },
                GuideBlock { kind: "bidon".into(), data: json!({}) },
            ],
            ..Default::default()
        });

        assert!(import(&mut conn, &dir, &g).is_err());
        assert!(db::list_sections(&conn, 620).unwrap().is_empty());
        assert!(db::list_free_cards(&conn, 620).unwrap().is_empty());
        assert!(db::list_blocks(&conn, 620).unwrap().is_empty());
        assert_eq!(std::fs::read_dir(dir.join("blocks")).unwrap().count(), 0, "image recopiée effacée");
        // La connexion reste utilisable après l'annulation
        db::create_free_card(&conn, 620, "Après").unwrap();
        let _ = std::fs::remove_dir_all(&dir);
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
            sections: vec![],
            cards: vec![],
            images: BTreeMap::new(),
        };
        assert!(check(&g).is_err());
        g.format = "chasse-succes-guide".into();
        assert!(check(&g).is_ok(), "guide exporté avant le changement de nom");
        g.format = FORMAT.into();
        g.version = VERSION + 1;
        assert!(check(&g).is_err());
        assert_eq!(file_name("Portal 2: \"édition\" <x>"), "Portal 2 édition x - guide.json");
        assert_eq!(file_name("Portal 2"), "Portal 2 - guide.json");
    }

    /// Un guide exporté quand les cartes avaient un statut (« column ») se lit toujours : le champ est ignoré.
    #[test]
    fn ancien_statut_ignore() {
        let p: GuidePlace =
            serde_json::from_str(r#"{"column":"doing","collapsed":true,"grid":null,"pinned":false}"#).unwrap();
        assert!(p.collapsed && p.grid.is_none());
    }
}
