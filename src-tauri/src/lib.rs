//! Cœur de l'application.
//! - `steam` : lecture des données Steam (lecture seule, isolé)
//! - `db` : base SQLite locale
//! - `library` : commandes de la bibliothèque de jeux
//! - `blocks` : blocs des guides et leurs images
//! - `tags` : étiquettes
//! - `guides` : export / import des guides d'un jeu
//! - `watch` : actualisation automatique quand Steam réécrit la progression d'un jeu
//! - `backup` : sauvegardes de sécurité des données
//! - `update` : mises à jour (installeur) ou annonce des nouvelles versions (version portable)

pub mod backup;
pub mod blocks;
pub mod data_dir;
pub mod db;
pub mod guides;
pub mod library;
pub mod steam;
pub mod tags;
pub mod update;
pub mod watch;

use std::sync::Mutex;
use tauri::utils::config::{AppDirectoriesOverride, AppDirectoryOverrides};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut context = tauri::generate_context!();
    // Version portable (`portable.txt` à côté de l'exe) : tous les dossiers de l'application, dont
    // celui du moteur web (%LOCALAPPDATA% sinon), dans `donnees\` à côté de l'exécutable
    let portable = data_dir::portable_dir();
    if let Some(dir) = &portable {
        context.config_mut().app.app_directories_override = Some(AppDirectoriesOverride::Directories(AppDirectoryOverrides {
            config: Some(dir.clone()),
            data: Some(dir.clone()),
            local_data: Some(dir.join("webview")),
            cache: Some(dir.join("webview").join("cache")),
            log: Some(dir.join("logs")),
        }));
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            // %APPDATA%\Grimoire sous Windows, ~/.local/share/Grimoire sous Linux (data_dir.rs) ;
            // `donnees\` à côté de l'exécutable pour la version portable
            let data_dir = match &portable {
                Some(dir) => dir.clone(),
                None => data_dir::resolve(&app.path().data_dir()?, &app.path().app_data_dir()?)?,
            };
            app.manage(update::UpdateState::new(portable.is_some()));
            let conn = db::open(&data_dir.join(data_dir::DB_FILE))?;
            let images_dir = data_dir.join("images");
            // Jaquettes et images des blocs servies par le protocole asset : ce dossier seulement
            app.asset_protocol_scope().allow_directory(&images_dir, true)?;
            blocks::cleanup_orphan_images(&conn, &images_dir);
            app.manage(library::AppState { db: Mutex::new(conn), data_dir, images_dir });
            watch::start(app.handle().clone());
            backup::start(app.handle().clone());
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
            backup::backups_list,
            backup::backup_create,
            backup::backup_restore,
            backup::backup_delete,
            backup::backups_open_folder,
            library::library_reorder,
            library::library_local_suggestions,
            library::library_search,
            library::library_import,
            library::library_delete,
            library::game_achievements,
            library::achievement_set_unlocked,
            library::achievements_lock_all,
            library::game_recap,
            library::home_recent_unlocks,
            library::game_mark_celebrated,
            library::workspace_list,
            library::workspace_save,
            library::workspace_card_sizes,
            library::sections_list,
            library::section_create,
            library::sections_save,
            library::section_delete,
            library::free_cards_list,
            library::free_card_create,
            library::free_card_rename,
            library::free_card_delete,
            library::setting_get,
            library::setting_set,
            blocks::blocks_list,
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
            guides::guide_export,
            guides::guide_import,
            update::app_mode,
            update::update_check,
            update::update_install,
            update::update_open_page,
        ])
        .run(context)
        .expect("erreur au lancement de l'application");
}
