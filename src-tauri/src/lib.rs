//! Cœur de l'application.
//! - `steam` : lecture des données Steam (lecture seule, isolé)
//! - `db` : base SQLite locale
//! - `library` : commandes de la bibliothèque de jeux
//! - `blocks` : blocs des guides et leurs images

pub mod blocks;
pub mod db;
pub mod library;
pub mod steam;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
        .invoke_handler(tauri::generate_handler![
            library::images_dir,
            library::library_list,
            library::library_local_suggestions,
            library::library_search,
            library::library_import,
            library::library_delete,
            library::game_achievements,
            library::achievement_set_unlocked,
            library::workspace_list,
            library::workspace_save,
            library::setting_get,
            library::setting_set,
            blocks::blocks_list,
            blocks::block_create,
            blocks::block_update,
            blocks::block_delete,
            blocks::blocks_reorder,
            blocks::image_save,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application");
}
