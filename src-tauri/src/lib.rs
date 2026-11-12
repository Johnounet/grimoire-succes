//! Cœur de l'application. Les modules (base de données, raccourcis…)
//! s'ajouteront ici au fil des étapes du SPEC.md.

pub mod steam;

/// Commande de démonstration : vérifie que l'interface communique avec le cœur Rust.
#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
fn steam_detect() -> steam::DetectReport {
    steam::detect_all()
}

#[tauri::command]
async fn steam_probe(appid: u32, lang: String, profile: Option<String>) -> steam::ProbeReport {
    let profile = profile.filter(|p| !p.trim().is_empty());
    steam::probe(appid, &lang, profile.as_deref()).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_version, steam_detect, steam_probe])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application");
}
