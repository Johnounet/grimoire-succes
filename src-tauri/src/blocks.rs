//! Blocs des guides (vidéo, image, texte, liste, compteur, chronomètre) et leurs images.

use crate::db::{self, Block};
use crate::library::AppState;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::State;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn blocks_list(state: State<'_, AppState>, appid: u32) -> Result<Vec<Block>, String> {
    db::list_blocks(&state.conn(), appid).map_err(err)
}

#[tauri::command]
pub fn block_create(
    state: State<'_, AppState>,
    achievement_id: i64,
    kind: String,
    data: serde_json::Value,
    position: Option<i64>,
) -> Result<Block, String> {
    db::create_block(&mut state.conn(), achievement_id, &kind, &data, position).map_err(err)
}

#[tauri::command]
pub fn block_update(state: State<'_, AppState>, id: i64, data: serde_json::Value) -> Result<(), String> {
    db::update_block(&state.conn(), id, &data).map_err(err)
}

/// Supprime un bloc. Son image éventuelle est gardée jusqu'au prochain démarrage,
/// pour que « Annuler » puisse le recréer à l'identique.
#[tauri::command]
pub fn block_delete(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    db::delete_block(&state.conn(), id).map_err(err)
}

#[tauri::command]
pub fn blocks_reorder(state: State<'_, AppState>, achievement_id: i64, ids: Vec<i64>) -> Result<(), String> {
    db::reorder_blocks(&mut state.conn(), achievement_id, &ids).map_err(err)
}

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];
const MAX_IMAGE_BYTES: usize = 25 * 1024 * 1024;

/// Reconnaît le format d'après les premiers octets (on ne se fie pas à l'extension annoncée).
fn sniff_extension(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some("png"),
        [0xFF, 0xD8, 0xFF, ..] => Some("jpg"),
        [b'G', b'I', b'F', b'8', ..] => Some("gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("webp"),
        [b'B', b'M', ..] => Some("bmp"),
        _ => None,
    }
}

/// Enregistre une image envoyée en binaire brut (corps de la requête) dans `images/blocks/`.
/// Renvoie son chemin relatif à `images/`, à mettre dans le bloc.
#[tauri::command]
pub fn image_save(state: State<'_, AppState>, request: tauri::ipc::Request<'_>) -> Result<String, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("image attendue en binaire brut".into());
    };
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err("image trop lourde (25 Mo au maximum)".into());
    }
    let ext = sniff_extension(bytes).ok_or("format d'image non reconnu (PNG, JPEG, GIF, WebP ou BMP)")?;
    debug_assert!(IMAGE_EXTENSIONS.contains(&ext));

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let rel = format!("blocks/{stamp}-{}.{ext}", COUNTER.fetch_add(1, Ordering::Relaxed));

    let path = state.images_dir.join(&rel);
    std::fs::create_dir_all(path.parent().expect("dossier parent")).map_err(err)?;
    std::fs::write(&path, bytes).map_err(err)?;
    Ok(rel)
}

/// Au démarrage : supprime les images de blocs que plus aucun bloc n'utilise.
pub fn cleanup_orphan_images(conn: &rusqlite::Connection, images_dir: &Path) {
    let Ok(used) = db::referenced_block_images(conn) else { return };
    let Ok(dir) = std::fs::read_dir(images_dir.join("blocks")) else { return };
    for entry in dir.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else { continue };
        if !used.contains(&format!("blocks/{name}")) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_les_formats() {
        assert_eq!(sniff_extension(b"\x89PNG\r\n\x1a\n...."), Some("png"));
        assert_eq!(sniff_extension(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(sniff_extension(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(sniff_extension(b"<svg onload=...>"), None);
    }
}
