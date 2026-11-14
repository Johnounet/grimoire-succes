//! Taille, position et état agrandi de la fenêtre principale, retenus d'un lancement à l'autre
//! dans `fenetre.json` (dossier des données).
//!
//! Pas de `tauri-plugin-window-state` : il écrit dans le dossier de l'identifiant de l'application
//! (`%APPDATA%\io.github.johnounet.grimoire`), que Grimoire n'utilise plus (voir `data_dir.rs`).
//!
//! La fenêtre est créée cachée (`tauri.conf.json`) : on lui applique l'état retenu, puis on
//! l'affiche. Premier lancement : agrandie. Position retenue sur un écran débranché depuis :
//! fenêtre centrée sur l'écran principal.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{PhysicalPosition, PhysicalSize, Runtime, WebviewWindow, WindowEvent};

const FILE: &str = "fenetre.json";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct State {
    /// Position et taille de la fenêtre non agrandie (pixels physiques)
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    maximized: bool,
}

fn load(path: &Path) -> Option<State> {
    let state: State = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    (state.width > 0 && state.height > 0).then_some(state)
}

/// Le centre de la fenêtre est-il sur l'un des écrans ?
fn on_a_monitor(state: &State, monitors: &[(PhysicalPosition<i32>, PhysicalSize<u32>)]) -> bool {
    let cx = state.x as i64 + state.width as i64 / 2;
    let cy = state.y as i64 + state.height as i64 / 2;
    monitors.iter().any(|(p, s)| {
        let (x, y) = (p.x as i64, p.y as i64);
        cx >= x && cx < x + s.width as i64 && cy >= y && cy < y + s.height as i64
    })
}

/// Applique l'état retenu à la fenêtre principale, l'affiche et suit ses changements.
pub fn restore_and_track<R: Runtime>(window: &WebviewWindow<R>, data_dir: &Path) -> tauri::Result<()> {
    let path = data_dir.join(FILE);
    let saved = load(&path);
    let monitors: Vec<_> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| (*m.position(), *m.size()))
        .collect();

    match saved {
        Some(s) => {
            // Taille d'abord : elle sert à centrer si la position n'est plus sur aucun écran
            window.set_size(PhysicalSize::new(s.width, s.height))?;
            if on_a_monitor(&s, &monitors) {
                window.set_position(PhysicalPosition::new(s.x, s.y))?;
            } else {
                window.center()?;
            }
            if s.maximized {
                window.maximize()?;
            }
        }
        None => window.maximize()?,
    }
    window.show()?;
    window.set_focus()?;

    // Position et taille « normales » : seulement quand la fenêtre n'est ni agrandie ni réduite
    let normal = Arc::new(Mutex::new(saved.map(|s| (s.x, s.y, s.width, s.height))));
    let snapshot = |w: &WebviewWindow<R>, normal: &Mutex<Option<(i32, i32, u32, u32)>>| {
        if w.is_maximized().unwrap_or(true) || w.is_minimized().unwrap_or(true) {
            return;
        }
        if let (Ok(p), Ok(s)) = (w.outer_position(), w.inner_size()) {
            *normal.lock().unwrap() = Some((p.x, p.y, s.width, s.height));
        }
    };
    snapshot(window, &normal);

    let w = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => snapshot(&w, &normal),
        WindowEvent::CloseRequested { .. } => {
            let Some((x, y, width, height)) = *normal.lock().unwrap() else { return };
            let state = State { x, y, width, height, maximized: w.is_maximized().unwrap_or(false) };
            save(&path, &state);
        }
        _ => {}
    });
    Ok(())
}

fn save(path: &PathBuf, state: &State) {
    if let Ok(json) = serde_json::to_vec_pretty(state) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(x: i32, y: i32, w: u32, h: u32) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
        (PhysicalPosition::new(x, y), PhysicalSize::new(w, h))
    }

    #[test]
    fn fenetre_hors_ecran_detectee() {
        let screens = [monitor(0, 0, 1920, 1080), monitor(1920, 0, 2560, 1440)];
        let at = |x, y| State { x, y, width: 1280, height: 800, maximized: false };
        assert!(on_a_monitor(&at(100, 100), &screens));
        assert!(on_a_monitor(&at(2000, 200), &screens), "second écran");
        // Agrandie, Windows place la fenêtre un peu au-delà du bord : toujours sur l'écran
        assert!(on_a_monitor(&State { x: -8, y: -8, width: 1936, height: 1048, maximized: true }, &screens));
        assert!(!on_a_monitor(&at(5000, 100), &screens), "écran débranché");
        assert!(!on_a_monitor(&at(-3000, 100), &screens));
    }

    #[test]
    fn fichier_absent_ou_abime_ignore() {
        let dir = std::env::temp_dir().join(format!("grimoire-test-fenetre-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE);
        assert_eq!(load(&path), None);
        std::fs::write(&path, b"{pas du json").unwrap();
        assert_eq!(load(&path), None);
        let state = State { x: 10, y: 20, width: 1280, height: 800, maximized: true };
        save(&path, &state);
        assert_eq!(load(&path), Some(state));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
