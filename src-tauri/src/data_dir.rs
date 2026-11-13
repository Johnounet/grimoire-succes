//! Dossier des données du joueur : `%APPDATA%\Grimoire` sous Windows (`~/.local/share/Grimoire`
//! sous Linux). Nom lisible, sans l'identifiant technique de l'application (qui contient le pseudo
//! GitHub de l'auteur).
//!
//! Les premières versions rangeaient les données dans le dossier de l'identifiant
//! (`%APPDATA%\io.github.johnounet.grimoire`) : elles sont déplacées au premier lancement.
//! « Grimoire » étant un nom courant, un dossier `Grimoire` qui existe déjà sans contenir de base
//! Grimoire n'est pas réutilisé : on prend alors `Grimoire (succès)`.

use std::path::{Path, PathBuf};

pub const DB_FILE: &str = "donnees.sqlite";
const NAMES: &[&str] = &["Grimoire", "Grimoire (succès)"];

/// Dossier utilisable : absent, vide, ou déjà celui de Grimoire.
fn usable(dir: &Path) -> bool {
    !dir.exists() || dir.join(DB_FILE).is_file() || std::fs::read_dir(dir).is_ok_and(|mut d| d.next().is_none())
}

/// Choisit le dossier des données dans `base` (dossier des données de l'utilisateur) et y déplace
/// celles de `old` (ancien dossier) s'il le faut.
pub fn resolve(base: &Path, old: &Path) -> std::io::Result<PathBuf> {
    let dir = NAMES
        .iter()
        .map(|n| base.join(n))
        .find(|d| usable(d))
        .unwrap_or_else(|| base.join(NAMES[NAMES.len() - 1]));

    if !dir.join(DB_FILE).exists() && old.join(DB_FILE).is_file() && old != dir {
        move_dir(old, &dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Déplace un dossier : renommage direct, sinon entrée par entrée (dossier d'arrivée déjà créé).
fn move_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    if !to.exists() && std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)?.flatten() {
        let dest = to.join(entry.file_name());
        if dest.exists() {
            continue;
        }
        if std::fs::rename(entry.path(), &dest).is_err() {
            // Autre disque : copie puis suppression
            copy_all(&entry.path(), &dest)?;
            let _ = if entry.path().is_dir() {
                std::fs::remove_dir_all(entry.path())
            } else {
                std::fs::remove_file(entry.path())
            };
        }
    }
    let _ = std::fs::remove_dir(from);
    Ok(())
}

fn copy_all(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for e in std::fs::read_dir(from)?.flatten() {
            copy_all(&e.path(), &to.join(e.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(from, to).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("grimoire-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn deplace_les_anciennes_donnees() {
        let base = temp("data-dir");
        let old = base.join("io.github.johnounet.grimoire");
        std::fs::create_dir_all(old.join("images/blocks")).unwrap();
        std::fs::write(old.join(DB_FILE), b"base").unwrap();
        std::fs::write(old.join("images/blocks/a.png"), b"img").unwrap();

        let dir = resolve(&base, &old).unwrap();
        assert_eq!(dir, base.join("Grimoire"));
        assert_eq!(std::fs::read(dir.join(DB_FILE)).unwrap(), b"base");
        assert!(dir.join("images/blocks/a.png").is_file());
        assert!(!old.exists());
        // Lancements suivants : même dossier, rien ne bouge
        assert_eq!(resolve(&base, &old).unwrap(), dir);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn ne_reprend_pas_le_dossier_d_un_autre_logiciel() {
        let base = temp("data-dir-autre");
        std::fs::create_dir_all(base.join("Grimoire")).unwrap();
        std::fs::write(base.join("Grimoire/config.ini"), b"autre logiciel").unwrap();
        let dir = resolve(&base, &base.join("ancien")).unwrap();
        assert_eq!(dir, base.join("Grimoire (succès)"));
        assert!(dir.is_dir());
        let _ = std::fs::remove_dir_all(&base);
    }
}
