//! Mises à jour de Grimoire, par les versions publiées sur GitHub (`latest.json`, adresse dans
//! `plugins.updater.endpoints` de `tauri.conf.json`).
//!
//! - Version installée : téléchargement, vérification de la signature (`plugins.updater.pubkey`)
//!   puis installeur NSIS lancé en mode passif ; l'application se ferme et redémarre.
//! - Version portable (ou clé publique pas encore renseignée, ou version de développement) :
//!   seulement l'annonce de la nouvelle version, avec un lien vers la page de téléchargement.

use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

/// Page des versions publiées (téléchargement manuel, notes de version).
pub const RELEASES_PAGE: &str = "https://github.com/Johnounet/grimoire-succes/releases/latest";

pub struct UpdateState {
    /// Version portable (`portable.txt` à côté de l'exécutable) : pas d'installation automatique.
    pub portable: bool,
    /// Dernière version trouvée, gardée pour l'installer sans revérifier.
    pending: Mutex<Option<Update>>,
}

impl UpdateState {
    pub fn new(portable: bool) -> Self {
        Self { portable, pending: Mutex::new(None) }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// Version proposée (ex. « 0.2.0 »).
    pub version: String,
    /// Notes de version, telles que publiées sur GitHub.
    pub notes: Option<String>,
    /// Installation en un clic possible ; sinon, lien vers la page de téléchargement.
    pub can_install: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMode {
    pub portable: bool,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Clé publique de signature renseignée dans `tauri.conf.json`.
fn has_pubkey(app: &AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("pubkey"))
        .and_then(|k| k.as_str())
        .is_some_and(|k| !k.trim().is_empty())
}

#[tauri::command]
pub fn app_mode(state: State<'_, UpdateState>) -> AppMode {
    AppMode { portable: state.portable }
}

/// Cherche une version plus récente. `None` : à jour.
#[tauri::command]
pub async fn update_check(app: AppHandle, state: State<'_, UpdateState>) -> Result<Option<UpdateInfo>, String> {
    let update = app.updater().map_err(err)?.check().await.map_err(err)?;
    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        notes: u.body.clone().filter(|b| !b.trim().is_empty()),
        can_install: !state.portable && !cfg!(debug_assertions) && has_pubkey(&app),
    });
    *state.pending.lock().map_err(err)? = update;
    Ok(info)
}

/// Télécharge et installe la version trouvée par `update_check`. Sous Windows, l'application se
/// ferme quand l'installeur démarre (l'interface a enregistré ses blocs avant l'appel).
#[tauri::command]
pub async fn update_install(app: AppHandle, state: State<'_, UpdateState>) -> Result<(), String> {
    if state.portable || cfg!(debug_assertions) || !has_pubkey(&app) {
        return Err("installation automatique indisponible pour cette version".into());
    }
    let update = state.pending.lock().map_err(err)?.clone().ok_or("aucune mise à jour trouvée")?;
    // Sous Windows, l'installeur ferme l'application sans fermeture des fenêtres : leur place
    // est enregistrée une fois le téléchargement fini, et la copie de fin de séance écrite
    let app_for_save = app.clone();
    update
        .download_and_install(|_, _| {}, move || {
            crate::window_state::save_all(&app_for_save);
            crate::backup::on_close_app(&app_for_save);
        })
        .await
        .map_err(err)?;
    // Hors Windows, l'installation ne relance pas l'application
    app.restart();
}

/// Ouvre la page de téléchargement des versions dans le navigateur.
#[tauri::command]
pub fn update_open_page(app: AppHandle) -> Result<(), String> {
    app.opener().open_url(RELEASES_PAGE, None::<&str>).map_err(err)
}
