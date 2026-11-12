//! Cœur de l'application. Les modules (Steam, base de données, raccourcis)
//! s'ajouteront ici au fil des étapes du SPEC.md.

/// Commande de démonstration : vérifie que l'interface communique avec le cœur Rust.
#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_version])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application");
}
