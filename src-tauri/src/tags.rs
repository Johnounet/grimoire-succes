//! Étiquettes : définies une fois pour tous les jeux (Réglages), posées sur les succès.

use crate::db::{self, AchievementTag, Tag};
use crate::library::AppState;
use tauri::State;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn tags_list(state: State<'_, AppState>) -> Result<Vec<Tag>, String> {
    db::list_tags(&state.conn()).map_err(err)
}

#[tauri::command]
pub fn tag_create(state: State<'_, AppState>, name: String, color: String) -> Result<Tag, String> {
    db::create_tag(&state.conn(), &name, &color)
}

#[tauri::command]
pub fn tag_update(state: State<'_, AppState>, id: i64, name: String, color: String) -> Result<(), String> {
    db::update_tag(&state.conn(), id, &name, &color)
}

#[tauri::command]
pub fn tag_delete(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    db::delete_tag(&state.conn(), id).map_err(err)
}

#[tauri::command]
pub fn achievement_tags_list(state: State<'_, AppState>, appid: u32) -> Result<Vec<AchievementTag>, String> {
    db::list_achievement_tags(&state.conn(), appid).map_err(err)
}

#[tauri::command]
pub fn achievement_tag_set(state: State<'_, AppState>, achievement_id: i64, tag_id: i64, on: bool) -> Result<(), String> {
    db::set_achievement_tag(&state.conn(), achievement_id, tag_id, on).map_err(err)
}
