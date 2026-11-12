//! Cœur de l'application.
//! - `steam` : lecture des données Steam (lecture seule, isolé)
//! - `db` : base SQLite locale
//! - `library` : commandes de la bibliothèque de jeux
//! - `blocks` : blocs des guides et leurs images
//! - `tags` : étiquettes

pub mod blocks;
pub mod db;
pub mod library;
pub mod steam;
pub mod tags;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // %APPDATA%\<identifiant> sous Windows, ~/.local/share/<identifiant> sous Linux
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open(&data_dir.join("donnees.sqlite"))?;
            let images_dir = data_dir.join("images");
            blocks::cleanup_orphan_images(&conn, &images_dir);
            app.manage(library::AppState { db: Mutex::new(conn), images_dir });
            Ok(())
        })
        // Fermer la fenêtre principale quitte l'application, fenêtre compacte comprise
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            library::images_dir,
            library::library_list,
            library::library_local_suggestions,
            library::library_search,
            library::library_import,
            library::library_delete,
            library::game_achievements,
            library::achievement_set_unlocked,
            library::achievements_lock_all,
            library::game_recap,
            library::game_mark_celebrated,
            library::workspace_list,
            library::workspace_save,
            library::workspace_card_sizes,
            library::setting_get,
            library::setting_set,
            blocks::blocks_list,
            blocks::block_get,
            blocks::block_create,
            blocks::block_update,
            blocks::block_delete,
            blocks::blocks_reorder,
            blocks::image_save,
            tags::tags_list,
            tags::tag_create,
            tags::tag_update,
            tags::tag_delete,
            tags::achievement_tags_list,
            tags::achievement_tag_set,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application");
}
