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
            let size = PhysicalSize::new(s.width, s.height);
            if on_a_monitor(&s, &monitors) {
                // Position d'abord : arrivée sur un écran à une autre échelle (100 % → 150 %),
                // la fenêtre est remise à l'échelle par Windows ; la taille retenue vient après.
                window.set_position(PhysicalPosition::new(s.x, s.y))?;
                window.set_size(size)?;
            } else {
                // Taille d'abord : elle sert à centrer sur l'écran où la fenêtre a été créée
                window.set_size(size)?;
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

    // Suivi tant que la fenêtre n'est pas réduite : état agrandi, et position et taille
    // « normales » (fenêtre ni agrandie ni réduite). Réduite, Windows la dit non agrandie :
    // fermée depuis la barre des tâches, elle doit garder l'état d'avant la réduction.
    let normal = saved.map(|s| (s.x, s.y, s.width, s.height));
    let tracked = Arc::new(Mutex::new((normal, saved.map_or(true, |s| s.maximized))));
    let snapshot = |w: &WebviewWindow<R>, tracked: &Mutex<(Option<(i32, i32, u32, u32)>, bool)>| {
        if w.is_minimized().unwrap_or(true) {
            return;
        }
        let Ok(maximized) = w.is_maximized() else { return };
        let mut t = tracked.lock().unwrap();
        t.1 = maximized;
        if !maximized {
            if let (Ok(p), Ok(s)) = (w.outer_position(), w.inner_size()) {
                t.0 = Some((p.x, p.y, s.width, s.height));
            }
        }
    };
    snapshot(window, &tracked);

    let w = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => snapshot(&w, &tracked),
        WindowEvent::CloseRequested { .. } => {
            let (normal, maximized) = *tracked.lock().unwrap();
            let Some((x, y, width, height)) = normal else { return };
            save(&path, &State { x, y, width, height, maximized });
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
