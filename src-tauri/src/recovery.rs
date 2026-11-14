//! Ouverture de la base au démarrage, et secours quand elle échoue.
//!
//! La version publiée n'a pas de console : une erreur au démarrage fermait la fenêtre sans un mot.
//! Ici, chaque échec est expliqué dans une boîte de dialogue du système (`rfd`, appelée sur le fil
//! principal : celles de `tauri-plugin-dialog` y bloqueraient, la boucle d'événements ne tournant
//! pas encore pendant `setup`).
//! - Base abîmée (SQLite : « pas une base » ou « image disque abîmée ») : elle est mise de côté
//!   (renommée, jamais supprimée) puis le joueur choisit : remettre la dernière sauvegarde,
//!   repartir de zéro, ouvrir le dossier des données ou quitter.
//! - Autre échec (migration défectueuse, fichier verrouillé…) : les données ne sont pas touchées ;
//!   message, dossier des données à portée, puis fermeture.

use crate::backup;
use crate::data_dir::DB_FILE;
use crate::db;
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use rusqlite::{Connection, ErrorCode};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub enum OpenError {
    /// Le fichier n'est pas (ou plus) une base lisible.
    Corrupt(String),
    /// Toute autre raison : la base elle-même n'est pas en cause, on n'y touche pas.
    Other(String),
}

fn classify(e: rusqlite::Error) -> OpenError {
    match &e {
        rusqlite::Error::SqliteFailure(f, _) if matches!(f.code, ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase) => {
            OpenError::Corrupt(e.to_string())
        }
        _ => OpenError::Other(e.to_string()),
    }
}

/// Ouvre la base, la sauvegarde si une nouvelle version du logiciel va la migrer, puis la migre.
pub fn open_database(data_dir: &Path, images_dir: &Path) -> Result<Connection, OpenError> {
    let mut conn = db::open_unmigrated(&data_dir.join(DB_FILE)).map_err(classify)?;
    // Première lecture : c'est elle qui révèle un fichier qui n'est pas une base
    db::needs_migration(&conn).map_err(classify)?;
    // Nouvelle version : copie avant que les migrations modifient la base.
    // Un échec (disque plein…) n'empêche pas de démarrer.
    if let Err(e) = backup::before_migration(&conn, data_dir, images_dir) {
        eprintln!("sauvegarde avant mise à jour : {e}");
    }
    db::migrate_existing(&mut conn).map_err(classify)?;
    Ok(conn)
}

/// Renomme la base abîmée (et ses fichiers de journal, qui ne doivent pas être appliqués à la base
/// qui la remplacera) en `donnees-abimee-<horodatage>.sqlite`. Renvoie le nouveau nom.
pub fn set_aside(data_dir: &Path) -> std::io::Result<PathBuf> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mut n = 0;
    let target = loop {
        let name = if n == 0 { format!("donnees-abimee-{stamp}.sqlite") } else { format!("donnees-abimee-{stamp}-{n}.sqlite") };
        let path = data_dir.join(name);
        if !path.exists() {
            break path;
        }
        n += 1;
    };
    let db = data_dir.join(DB_FILE);
    if db.exists() {
        std::fs::rename(&db, &target)?;
    }
    for suffix in ["-wal", "-shm"] {
        let side = data_dir.join(format!("{DB_FILE}{suffix}"));
        if side.exists() {
            let mut to = target.clone().into_os_string();
            to.push(suffix);
            std::fs::rename(&side, PathBuf::from(to))?;
        }
    }
    Ok(target)
}

/// Ouvre la base ; en cas d'échec, explique et propose les solutions. Ne revient pas si le joueur
/// choisit de quitter.
pub fn open_or_recover(app: &tauri::App, data_dir: &Path, images_dir: &Path) -> Connection {
    match open_database(data_dir, images_dir) {
        Ok(conn) => conn,
        Err(OpenError::Other(e)) => {
            let text = format!(
                "Grimoire n'a pas pu ouvrir ses données.\n\nDétail : {e}\n\n\
                 Vos données n'ont pas été effacées. Si ce problème est apparu juste après une mise à jour, \
                 une copie de vos données d'avant la mise à jour se trouve dans le dossier « backups »."
            );
            if ask(&text, MessageButtons::OkCancelCustom("Ouvrir le dossier".into(), "Quitter".into())) == 0 {
                open_folder(app, data_dir);
            }
            std::process::exit(1);
        }
        Err(OpenError::Corrupt(e)) => recover(app, data_dir, images_dir, &e),
    }
}

fn recover(app: &tauri::App, data_dir: &Path, images_dir: &Path, detail: &str) -> Connection {
    let aside = match set_aside(data_dir) {
        Ok(p) => p,
        Err(err) => fatal(&format!(
            "Les données de Grimoire sont abîmées ({detail}) et n'ont pas pu être mises de côté ({err}).\n\n\
             Fermez les autres programmes qui pourraient utiliser le dossier des données, puis relancez Grimoire."
        )),
    };
    let aside_name = aside.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut text = format!(
        "Les données de Grimoire sont abîmées et ne peuvent pas être lues.\n\nDétail : {detail}\n\n\
         Le fichier abîmé est gardé de côté, sous le nom « {aside_name} » : il n'est pas supprimé.\n\n"
    );
    let mut has_backups = !backup::list(data_dir).is_empty();
    loop {
        let (first, question) = if has_backups {
            ("Restaurer la dernière sauvegarde", "Grimoire peut remettre la sauvegarde la plus récente.")
        } else {
            ("Repartir de zéro", "Aucune sauvegarde utilisable. Repartir de zéro crée une bibliothèque vide.")
        };
        let choice = ask(
            &format!("{text}{question}"),
            MessageButtons::YesNoCancelCustom(first.into(), "Ouvrir le dossier".into(), "Quitter".into()),
        );
        match choice {
            0 if has_backups => match backup::restore_latest_offline(data_dir, images_dir) {
                Ok(conn) => return conn,
                Err(e) => {
                    // Plus rien à proposer que repartir de zéro
                    has_backups = false;
                    text = format!("Aucune sauvegarde n'a pu être remise ({e}).\n\n");
                }
            },
            0 => match open_database(data_dir, images_dir) {
                Ok(conn) => return conn,
                Err(OpenError::Corrupt(e) | OpenError::Other(e)) => fatal(&format!("Impossible de créer une base vide : {e}")),
            },
            1 => {
                open_folder(app, data_dir);
                std::process::exit(1);
            }
            _ => std::process::exit(1),
        }
    }
}

/// Toute autre erreur au démarrage : message, puis fermeture.
pub fn fatal(text: &str) -> ! {
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title("Grimoire")
        .set_description(format!("Grimoire n'a pas pu démarrer.\n\n{text}"))
        .set_buttons(MessageButtons::Ok)
        .show();
    std::process::exit(1);
}

/// Boîte de dialogue à boutons personnalisés ; renvoie le rang du bouton choisi (fermée : dernier).
fn ask(text: &str, buttons: MessageButtons) -> usize {
    let labels: Vec<String> = match &buttons {
        MessageButtons::OkCancelCustom(a, b) => vec![a.clone(), b.clone()],
        MessageButtons::YesNoCancelCustom(a, b, c) => vec![a.clone(), b.clone(), c.clone()],
        _ => Vec::new(),
    };
    let last = labels.len().saturating_sub(1);
    let result = MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title("Grimoire")
        .set_description(text)
        .set_buttons(buttons)
        .show();
    match result {
        MessageDialogResult::Custom(label) => labels.iter().position(|l| *l == label).unwrap_or(last),
        // Linux : rfd peut renvoyer les boutons standard au lieu des libellés
        MessageDialogResult::Ok | MessageDialogResult::Yes => 0,
        MessageDialogResult::No if labels.len() == 3 => 1,
        _ => last,
    }
}

fn open_folder(app: &tauri::App, data_dir: &Path) {
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_path(data_dir.to_string_lossy(), None::<&str>);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("grimoire-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn base_abimee_reconnue_mise_de_cote_et_restauree() {
        let data = temp_dir("secours");
        let images = data.join("images");

        // Une vraie base, sauvegardée
        let conn = open_database(&data, &images).unwrap();
        conn.execute_batch(
            "INSERT INTO games (appid, name, source, language, imported_at, refreshed_at) VALUES (620, 'P', 'local_cache', 'french', 0, 0);",
        )
        .unwrap();
        backup::create(&conn, &data, &images, backup::Kind::Manuel).unwrap();
        drop(conn);

        // Puis remplacée par un fichier quelconque, et par des zéros
        for junk in [b"ceci n'est pas une base de donnees SQLite, pas du tout".repeat(100), vec![0u8; 16384]] {
            std::fs::write(data.join(DB_FILE), &junk).unwrap();
            assert!(matches!(open_database(&data, &images), Err(OpenError::Corrupt(_))));
        }

        // Mise de côté : le fichier abîmé est gardé, la place est libre
        std::fs::write(data.join(format!("{DB_FILE}-wal")), b"journal").unwrap();
        let aside = set_aside(&data).unwrap();
        assert_eq!(std::fs::read(&aside).unwrap(), vec![0u8; 16384]);
        assert!(!data.join(DB_FILE).exists() && !data.join(format!("{DB_FILE}-wal")).exists());
        assert!(data.join(format!("{}-wal", aside.file_name().unwrap().to_string_lossy())).exists());

        // Restauration de la dernière sauvegarde
        let conn = backup::restore_latest_offline(&data, &images).unwrap();
        assert_eq!(db::library_appids(&conn).unwrap(), vec![620]);
        drop(conn);
        let _ = std::fs::remove_dir_all(&data);
    }
}
