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
    // Base qui s'ouvre mais dont des pages sont abîmées : elle afficherait les jeux puis échouerait
    // à lire leurs succès. Traitée comme une base abîmée (contrôle de quelques millisecondes).
    if let Some(problem) = db::quick_check(&conn).map_err(classify)? {
        return Err(OpenError::Corrupt(problem.lines().next().unwrap_or_default().to_string()));
    }
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

/// Bibliothèque disparue : pas de base, ou une base neuve (fichier vidé, ou base abîmée mise de côté
/// à un lancement précédent où le joueur a choisi « Quitter »), alors que des sauvegardes contenant
/// des jeux existent. Sans cette question, Grimoire créait une bibliothèque vide sans rien dire.
/// Une base déjà créée par Grimoire (même sans jeu) n'est pas « neuve » : la question n'est posée
/// qu'une fois.
pub fn library_lost(data_dir: &Path) -> bool {
    if !backup::list(data_dir).iter().any(|b| b.games > 0) {
        return false;
    }
    let path = data_dir.join(DB_FILE);
    if !path.exists() {
        return true;
    }
    // Fichier illisible : pas « neuf », `open_database` le reconnaîtra comme abîmé
    Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .and_then(|c| db::is_blank(&c))
        .unwrap_or(false)
}

/// Propose de remettre la dernière sauvegarde à la place d'une bibliothèque disparue. `None` : le
/// joueur préfère une bibliothèque vide (ou aucune sauvegarde n'a pu être remise). Ne revient pas
/// s'il choisit de quitter.
fn offer_backup(data_dir: &Path, images_dir: &Path) -> Option<Connection> {
    let choice = ask(
        "Grimoire ne trouve plus vos données : la bibliothèque est vide, alors que des sauvegardes existent.\n\n\
         Grimoire peut remettre la sauvegarde la plus récente. Sinon, il démarre avec une bibliothèque vide ; \
         les sauvegardes restent disponibles dans Réglages → Sauvegardes.",
        MessageButtons::YesNoCancelCustom("Restaurer la dernière sauvegarde".into(), "Bibliothèque vide".into(), "Quitter".into()),
    );
    match choice {
        0 => {
            // Le fichier vide et surtout un journal resté d'une ancienne base sont écartés : ce
            // journal ne doit pas s'appliquer à la base remise
            if let Err(e) = set_aside(data_dir) {
                fatal(&format!("Les anciens fichiers de données n'ont pas pu être mis de côté ({e})."));
            }
            match backup::restore_latest_offline(data_dir, images_dir) {
                Ok(conn) => Some(conn),
                Err(e) => {
                    inform(&format!(
                        "Aucune sauvegarde n'a pu être remise ({e}).\n\nGrimoire démarre avec une bibliothèque vide. \
                         Les sauvegardes n'ont pas été supprimées."
                    ));
                    None
                }
            }
        }
        1 => None,
        _ => std::process::exit(1),
    }
}

/// Ouvre la base ; en cas d'échec, explique et propose les solutions. Ne revient pas si le joueur
/// choisit de quitter.
pub fn open_or_recover(app: &tauri::App, data_dir: &Path, images_dir: &Path) -> Connection {
    if library_lost(data_dir) {
        if let Some(conn) = offer_backup(data_dir, images_dir) {
            return conn;
        }
    }
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

/// Simple message, puis l'application continue.
fn inform(text: &str) {
    MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title("Grimoire")
        .set_description(text)
        .set_buttons(MessageButtons::Ok)
        .show();
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

    fn remove_db(data: &Path) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(data.join(format!("{DB_FILE}{suffix}")));
        }
    }

    const GAME: &str =
        "INSERT INTO games (appid, name, source, language, imported_at, refreshed_at) VALUES (620, 'P', 'local_cache', 'french', 0, 0);";

    /// Base absente ou neuve alors que des sauvegardes existent : la restauration est proposée,
    /// une seule fois (une base créée par Grimoire, même sans jeu, n'est plus « neuve »).
    #[test]
    fn bibliotheque_disparue_reconnue() {
        let data = temp_dir("disparue");
        let images = data.join("images");
        assert!(!library_lost(&data), "premier lancement : ni base ni sauvegarde");

        let conn = open_database(&data, &images).unwrap();
        // Sauvegarde d'une bibliothèque sans jeu : rien à proposer
        backup::create(&conn, &data, &images, backup::Kind::Manuel).unwrap();
        drop(conn);
        remove_db(&data);
        assert!(!library_lost(&data));

        let conn = open_database(&data, &images).unwrap();
        conn.execute_batch(GAME).unwrap();
        backup::create(&conn, &data, &images, backup::Kind::Manuel).unwrap();
        assert!(!library_lost(&data), "base en place");
        drop(conn);

        // Base abîmée mise de côté à un lancement précédent, puis « Quitter »
        remove_db(&data);
        assert!(library_lost(&data));
        // Fichier vidé (0 octet)
        std::fs::write(data.join(DB_FILE), b"").unwrap();
        assert!(library_lost(&data));
        // Fichier abîmé : c'est l'autre secours qui s'en occupe
        std::fs::write(data.join(DB_FILE), vec![0x42u8; 8192]).unwrap();
        assert!(!library_lost(&data));
        assert!(matches!(open_database(&data, &images), Err(OpenError::Corrupt(_))));

        // « Bibliothèque vide » choisie : la base est créée, la question ne revient pas
        remove_db(&data);
        drop(open_database(&data, &images).unwrap());
        assert!(!library_lost(&data));
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Base qui s'ouvre mais dont des pages sont abîmées : reconnue au démarrage, et jamais remise
    /// à la place des données quand c'est une sauvegarde.
    #[test]
    fn base_a_moitie_abimee_reconnue() {
        let data = temp_dir("a-moitie");
        let images = data.join("images");
        let mut conn = open_database(&data, &images).unwrap();
        conn.execute_batch(GAME).unwrap();
        let good = backup::create(&conn, &data, &images, backup::Kind::Manuel).unwrap();
        for i in 0..400 {
            conn.execute(
                "INSERT INTO achievements (appid, key, name, steam_order) VALUES (620, ?1, ?2, ?3)",
                rusqlite::params![format!("K{i}"), "x".repeat(300), i],
            )
            .unwrap();
        }
        let _: i64 = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0)).unwrap();
        assert_eq!(db::quick_check(&conn).unwrap(), None);

        // Sauvegarde plus récente, puis ses pages du milieu écrasées (le début du fichier reste sain)
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let bad = backup::create(&conn, &data, &images, backup::Kind::Manuel).unwrap();
        let damage = |path: &Path| {
            let mut bytes = std::fs::read(path).unwrap();
            let n = bytes.len();
            bytes[n / 2..n / 2 + 16384].fill(0x99);
            std::fs::write(path, bytes).unwrap();
        };
        damage(&backup::backups_dir(&data).join(&bad.id).join(DB_FILE));
        let refused = backup::restore(&mut conn, &data, &images, &bad.id).unwrap_err();
        assert!(refused.contains("abîmée"), "{refused}");
        assert_eq!(db::list_achievements(&conn, 620).unwrap().len(), 400, "données actuelles intactes");
        drop(conn);

        // La base elle-même abîmée de la même façon : reconnue, mise de côté, sauvegarde saine remise
        damage(&data.join(DB_FILE));
        assert!(matches!(open_database(&data, &images), Err(OpenError::Corrupt(_))));
        set_aside(&data).unwrap();
        let conn = backup::restore_latest_offline(&data, &images).unwrap();
        assert_eq!(db::library_appids(&conn).unwrap(), vec![620]);
        assert_eq!(db::list_achievements(&conn, 620).unwrap().len(), 0, "sauvegarde saine ({}), pas l'abîmée", good.id);
        drop(conn);
        let _ = std::fs::remove_dir_all(&data);
    }
}
