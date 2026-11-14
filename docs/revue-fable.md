# Revue de Grimoire (Claude Fable 5.2, 2026-11-14)

Revue en lecture seule de la version 0.2.0 (commit `8f8acd4`). Aucun fichier du projet n'a été modifié, à part ce rapport.

## Ce qui a été fait

- Lecture de tout le code Rust (`src-tauri/src/`) et de l'essentiel de l'interface (`src/`), de la configuration Tauri, des permissions et des deux workflows GitHub.
- `cargo test --lib` : **43 tests réussis**. `npm run check` : **0 erreur, 0 avertissement**.
- Interface simulée (`npm run dev`) : accueil, bibliothèque vide (`?empty`), fenêtre compacte (`?compact`), sections, menus, Réglages au clavier. Serveur arrêté à la fin.
- **Tests de confirmation** écrits sur une copie du cœur Rust placée hors du projet (dossier temporaire). Ils réutilisent le dossier de compilation `src-tauri/target/` pour ne pas tout recompiler : quelques fichiers de compilation en plus s'y trouvent, sans effet (dossier ignoré par git).
- Non testé : l'application réelle (`npm run tauri dev` interdit ici), donc tout ce qui touche aux vraies fenêtres, à WebView2, à l'installeur et aux vrais fichiers de Steam.

**Degrés de certitude utilisés**

- **Certain** : reproduit par un test ou dans l'interface simulée.
- **Très probable** : lu dans le code sans ambiguïté, mais pas exécuté.
- **Probable** / **À vérifier** : dépend d'un comportement de Windows, de Steam ou d'une bibliothèque que je n'ai pas pu observer.

## L'essentiel en 6 lignes

1. **Importer un guide peut perdre des blocs** : deux images sans légende dans la même carte, la seconde disparaît (bug 1).
2. **Un fichier Steam rempli de zéros fait planter Grimoire** d'un coup, sans message (bug 2).
3. **Une base abîmée empêche le démarrage, sans aucun message** (bug 3), et **aucune sauvegarde n'est faite avant une migration** (risque 3).
4. **Un guide mal formé casse l'affichage de l'espace de travail**, et on ne peut plus le réparer depuis l'interface (bug 4).
5. **L'espace de travail d'un jeu peut être vidé** par un enregistrement arrivé au mauvais moment (risque 1) : rare, mais la conséquence est lourde.
6. Le reste est solide : séparation Steam / base / interface propre, lecture seule de Steam respectée, import « tout ou rien », sauvegardes bien pensées, permissions serrées.

---

## 1. Bugs confirmés

### 1. Import d'un guide : les blocs identiques d'une même carte sont perdus (corrigé)

- **Où** : [guides.rs:454-467](../src-tauri/src/guides.rs#L454), fonction `import_all`, avec `comparable` ([guides.rs:296](../src-tauri/src/guides.rs#L296)).
- **Explication** : pour ne pas doubler un bloc quand on réimporte le même fichier, chaque bloc importé est comparé à ceux « déjà présents ». Mais chaque bloc qui vient d'être créé est ajouté à cette liste (ligne 466). Résultat : le deuxième bloc identique **du même fichier** est pris pour un doublon. Une image se compare par sa légende seule : deux captures sans légende sont donc « identiques ».
- **Cas touchés** : deux images sans légende (très courant), deux compteurs au même libellé dans un guide vierge (les valeurs sont remises à 0, donc égales), deux listes vides, deux textes égaux.
- **Reproduire** : une carte avec 2 images sans légende → Exporter le guide → l'importer sur une bibliothèque où le jeu n'a pas ces blocs (autre PC, ou après avoir supprimé puis rajouté le jeu). Il manque une image, et le bilan annonce « 1 élément déjà présent, ignoré ».
- **Test** : 6 blocs exportés (2 images, 2 compteurs, 2 listes) → 3 importés, 3 comptés comme doublons.
- **Piste** : comparer seulement aux blocs présents **avant** l'import (ne pas ajouter à `existing`), ou compter les exemplaires.
- **Certitude** : certain. **Gravité** : haute (perte silencieuse dans la fonction faite pour sauvegarder et partager).

### 2. Un fichier Steam rempli de zéros fait planter l'application (corrigé)

- **Où** : [kv_binary.rs:72-105](../src-tauri/src/steam/kv_binary.rs#L72), fonction `entries` (elle s'appelle elle-même ligne 86, sans limite de profondeur). Appelée par `local_cache::load` ([local_cache.rs:261](../src-tauri/src/steam/local_cache.rs#L261) et [270](../src-tauri/src/steam/local_cache.rs#L270)), donc par l'import, « Actualiser » et la surveillance ([watch.rs:86](../src-tauri/src/watch.rs#L86)).
- **Explication** : dans ce format, l'octet 0 veut dire « début d'un groupe ». Un fichier plein de zéros est lu comme des milliers de groupes emboîtés ; le programme s'enfonce jusqu'à épuiser sa mémoire de travail (la « pile »). Ce type de plantage ne se rattrape pas : **toute l'application se ferme**, sans message. Or un fichier rempli de zéros est exactement ce que Windows laisse parfois après une coupure de courant ou un plantage pendant une écriture.
- **Reproduire** : remplacer (sur une copie de test, pas dans le vrai Steam) un `UserGameStats_<compte>_<appid>.bin` ou un `UserGameStatsSchema_<appid>.bin` par un fichier de 16 Ko de zéros, puis importer ou actualiser ce jeu.
- **Test** : en version optimisée (comme la version publiée), 4 000 octets nuls → erreur propre ; **16 000 octets nuls → plantage** (`STATUS_STACK_OVERFLOW`). En version de développement, 2 000 octets suffisent.
- **Piste** : limiter la profondeur (64 niveaux suffisent largement) et renvoyer une erreur au-delà.
- **Certitude** : certain pour le plantage ; la fréquence réelle d'un tel fichier est faible. **Gravité** : haute quand ça arrive (le jeu concerné fait planter à chaque actualisation).

### 3. Base de données abîmée ou illisible : Grimoire ne démarre plus, sans message

- **Où** : [lib.rs:66](../src-tauri/src/lib.rs#L66) (`db::open(...)?` dans `setup`) et [lib.rs:138-139](../src-tauri/src/lib.rs#L138) (`.expect("erreur au lancement…")`). Même chemin pour une migration qui échoue ([db.rs:253-262](../src-tauri/src/db.rs#L253)).
- **Explication** : si `donnees.sqlite` est illisible, l'ouverture échoue, le démarrage s'arrête, et la version publiée n'a pas de console : la fenêtre apparaît un instant puis disparaît. Le joueur ne sait pas quoi faire, alors que ses sauvegardes sont juste à côté dans `backups/`.
- **Reproduire** : fermer Grimoire, remplacer `%APPDATA%\Grimoire\donnees.sqlite` par un fichier quelconque (après l'avoir mis de côté), relancer.
- **Test** : `db::open` renvoie bien `file is not a database` sur un fichier abîmé ou plein de zéros. La suite (fermeture sans message) est lue dans le code, pas observée.
- **Piste** : en cas d'échec, afficher une boîte de dialogue (le module `tauri-plugin-dialog` est déjà là) : « base illisible », avec les choix « restaurer la dernière sauvegarde » et « ouvrir le dossier ». Mettre la base abîmée de côté plutôt que l'écraser.
- **Certitude** : certain pour l'échec d'ouverture, très probable pour l'absence de message.

### 4. Un bloc mal formé (guide importé) casse l'affichage de l'espace de travail

- **Où** : [VideoBlock.svelte:24](../src/lib/blocks/VideoBlock.svelte#L24) et [CompactVideo.svelte:40](../src/lib/blocks/CompactVideo.svelte#L40) (`[...block.data.markers]`), [TextBlock.svelte:15](../src/lib/blocks/TextBlock.svelte#L15) (`renderMarkdown(block.data.markdown)`). Côté Rust, l'import ne vérifie que le **type** du bloc, pas son contenu : [guides.rs:454-467](../src-tauri/src/guides.rs#L454), [db.rs:1018-1027](../src-tauri/src/db.rs#L1018).
- **Explication** : l'interface suppose que chaque bloc a tous ses champs (une vidéo a toujours `markers`, un texte toujours `markdown`). Un fichier de guide modifié à la main, produit par un autre outil ou malveillant peut contenir `{"kind":"video","data":{"url":"…"}}`. L'import l'accepte ; à l'affichage, une erreur arrête le rendu des cartes. Le bloc fautif étant en base, **le problème revient à chaque ouverture du jeu**, et on ne peut pas le supprimer depuis l'interface.
- **Reproduire** : dans un guide exporté, retirer `"markers": []` d'un bloc vidéo, importer, ouvrir le jeu.
- **Test** : côté Rust, l'import accepte `{}`, une chaîne et même `null` comme données de bloc (5 blocs sur 5 importés). Dans l'interface simulée : une vidéo sans `markers` → `block.data.markers is not iterable`, 0 carte affichée, et « + Carte libre » n'affiche plus rien non plus, même sur un autre jeu, jusqu'au rechargement.
- **Risque voisin** : le même plantage arrivera le jour où une version ajoutera un champ obligatoire à un type de bloc (les anciens blocs ne l'auront pas).
- **Piste** : une fonction « remise en forme » par type de bloc, appliquée au chargement (`loadBlocks`) et à l'import : champ manquant ou du mauvais type → valeur par défaut de `defaultData`.
- **Certitude** : certain. **Gravité** : moyenne (il faut un fichier anormal), mais la CSP montre que les guides importés sont déjà traités comme non sûrs : c'est le trou qui reste.

### 5. Le badge « DLC » d'un succès est effacé par l'actualisation

- **Où** : [db.rs:431](../src-tauri/src/db.rs#L431) (`dlc_appid = excluded.dlc_appid` dans `save_game`), [local_cache.rs:201](../src-tauri/src/steam/local_cache.rs#L201) (le cache local ne connaît jamais le DLC), affichage dans [AchievementItem.svelte:120](../src/lib/AchievementItem.svelte#L120).
- **Explication** : seules les pages web de Steam disent quel DLC apporte un succès. Le cache local, qui passe en premier, ne le sait pas et envoie « aucun DLC » : la valeur en base est écrasée. Pour la rareté et la progression, le code garde la dernière valeur connue (`COALESCE`) ; pour le DLC, non. En pratique, un joueur qui a Steam sur son PC ne voit presque jamais le badge.
- **Reproduire** : importer un jeu à DLC jamais lancé sur ce PC (source : profil) → badges « DLC » présents ; lancer le jeu une fois, « Actualiser » → badges disparus.
- **Test** : `dlc = Some(999)` avant, `None` après une actualisation depuis le cache local.
- **Piste** : `COALESCE(excluded.dlc_appid, achievements.dlc_appid)`, et compléter le cache local avec le DLC de la page publique comme c'est déjà fait pour la rareté ([mod.rs:76-80](../src-tauri/src/steam/mod.rs#L76)).
- **Certitude** : certain.

### 6. Fenêtres Réglages, À propos, Export, Re-lock : Échap ne marche pas et le clavier passe derrière

- **Où** : [SettingsDialog.svelte:94-101](../src/lib/SettingsDialog.svelte#L94), [AboutDialog.svelte:40-47](../src/lib/AboutDialog.svelte#L40), [ExportDialog.svelte:45-50](../src/lib/ExportDialog.svelte#L45), [RelockDialog.svelte:74-81](../src/lib/RelockDialog.svelte#L74).
- **Explication** : à l'ouverture, le focus clavier reste sur le bouton qui a ouvert la fenêtre, **derrière** elle. Or Échap est écouté sur la fenêtre elle-même : tant qu'on n'a pas cliqué dedans, Échap ne fait rien. La touche Tab parcourt d'abord tous les boutons de l'arrière-plan. Un lecteur d'écran annonce une fenêtre modale qui ne l'est pas vraiment.
- **Reproduire** (interface simulée) : cliquer sur « Réglages », appuyer sur Échap → la fenêtre reste ouverte. Après le clic, l'élément actif est toujours le bouton « Réglages ».
- **Piste** : faire comme « Ajouter un jeu » ([AddGameDialog.svelte:26](../src/lib/AddGameDialog.svelte#L26)), qui utilise la balise `<dialog>` avec `showModal()` : focus, Échap et blocage de l'arrière-plan sont alors gérés par le navigateur. « 100 % » ([Celebration.svelte:52-58](../src/lib/Celebration.svelte#L52)) s'en sort déjà bien.
- **Certitude** : certain pour Réglages ; très probable pour les trois autres (même construction, aucun `focus()` dans leur code).

### 7. Menu « ⋯ » d'une carte : erreur si deux sections portent le même nom

- **Où** : [MoreMenu.svelte:62](../src/lib/MoreMenu.svelte#L62) (`{#each items as it, i (it.label)}`), libellés construits dans [Workspace.svelte:281-284](../src/lib/Workspace.svelte#L281).
- **Explication** : les entrées du menu sont repérées par leur texte. Deux sections sans titre donnent deux fois « Déplacer vers « Section sans titre » » : Svelte refuse les repères en double. Or « + Section » crée justement des sections sans titre.
- **Reproduire** (interface simulée) : « + Carte libre », trois fois « + Section » sans les nommer, puis menu « ⋯ » de la carte → erreur `each_key_duplicate`, le menu ne s'ouvre pas.
- **Certitude** : certain en mode développement. **À vérifier dans la version publiée** : ce contrôle de Svelte n'existe qu'en développement, le menu s'y ouvre probablement, mais les repères en double restent une faute. Correctif d'une ligne : repérer par le rang (`(i)`).

### 8. Langue, pays et lien de profil : des réglages lus, mais aucun moyen de les changer

- **Où** : lus dans [library.rs:25-36](../src-tauri/src/library.rs#L25) et autorisés dans `SETTINGS` ([library.rs:348-350](../src-tauri/src/library.rs#L348)), déclarés dans [library.ts:149-151](../src/lib/library.ts#L149), mais **jamais utilisés par l'interface** (aucune occurrence dans `src/`, hors types).
- **Conséquences** :
  - **Lien de profil** : sur un PC sans Steam, il est impossible d'indiquer son profil. Les jeux sont importés depuis la page publique, tous les succès « non obtenus », à cocher à la main. `docs/choix.md` (« Sources Steam ») parle pourtant du « lien de profil réglé ».
  - **Pays** : toujours `FR`. Sans gravité pour un joueur français ; la recherche du magasin dépend du pays.
  - **Langue** : toujours `french` (cohérent avec « traduction en dernier »).
- **Certitude** : certain (recherche dans tout `src/`). C'est un manque plus qu'un bug : à décider si le champ « profil Steam » revient dans les Réglages avant la traduction. À mon avis oui, c'est le seul recours du cas « Steam absent ».

### 9. Petits bugs confirmés

- **Recherche sans lettre ni chiffre** : [library.rs:85-88](../src-tauri/src/library.rs#L85) et [169-178](../src-tauri/src/library.rs#L169). Taper `!!` ou `??` donne une recherche vide après nettoyage, et « tout contient le vide » : les 10 premiers jeux du PC s'affichent. Test : `name_matches("Portal 2", "")` vaut vrai. Certain, sans gravité.
- **Erreurs d'étiquettes jamais affichées** : [tags.svelte.ts:22-24](../src/lib/tags.svelte.ts#L22) range l'erreur dans `tags.error`, que personne n'affiche. Une étiquette qui ne s'enregistre pas se décoche toute seule, sans explication. Certain (aucune lecture de `tags.error` dans `src/`).
- **Recherche par AppID hors ligne** : [library.rs:164](../src-tauri/src/library.rs#L164) répond « Aucun jeu Steam ne porte l'AppID … » alors que c'est le réseau qui manque. Très probable (lu, non exécuté).

---

## 2. Risques probables

### 1. L'espace de travail d'un jeu peut être vidé par un enregistrement mal tombé

- **Où** : [GameView.svelte:149-157](../src/lib/GameView.svelte#L149) (`saveWorkspace` prend `game.appid` au moment de l'appel), [Workspace.svelte:250-260](../src/lib/Workspace.svelte#L250) (`remove` attend la fin de l'animation avant d'enregistrer), chargement dans [GameView.svelte:101-131](../src/lib/GameView.svelte#L101). Côté Rust : [db.rs:770-772](../src-tauri/src/db.rs#L770) (`save_workspace` efface tout puis réécrit).
- **Explication** : enregistrer l'espace = « effacer toutes les cartes du jeu, puis écrire la liste reçue ». Si la liste reçue est vide ou vient d'un autre jeu, tout est effacé. Deux moments où ça peut arriver :
  1. **Retirer une carte puis changer de jeu tout de suite** : `remove` attend l'animation de sortie (~180 ms), puis enregistre `items`… qui est déjà la liste du nouveau jeu, vide tant qu'elle n'est pas chargée. Le nouveau jeu reçoit une liste vide.
  2. **Agir pendant le chargement** : entre l'ouverture d'un jeu et l'arrivée de ses cartes, `workspace` vaut `[]`. Glisser un succès à ce moment enregistre « une seule carte ». La fenêtre est de quelques millisecondes, mais elle s'allonge si le cœur est occupé (sauvegarde automatique du démarrage, voir risque 5).
- **Ce qui limite les dégâts** : les blocs ne sont jamais perdus (ils sont rattachés au succès), les cartes libres reviennent en fin d'espace ; ce sont la disposition, les sections des cartes et les épingles qui partent. L'affichage peut rester correct alors que la base est vide : on le découvre au lancement suivant.
- **Test** : côté Rust, enregistrer la liste d'un autre jeu sous le jeu 730 laisse bien **0 carte**. L'enchaînement côté interface est lu, pas reproduit (il demande un clic dans une fenêtre de 180 ms).
- **Piste** : passer l'`appid` avec la liste (`onchange(appid, items)`), refuser d'enregistrer tant que l'espace n'est pas chargé (drapeau `loaded`), et dans `remove` vérifier que le jeu n'a pas changé après l'attente.
- **Certitude** : probable (mécanisme certain, déclenchement rare).

### 2. Deux Grimoire ouverts en même temps

- **Où** : [lib.rs:45-50](../src-tauri/src/lib.rs#L45) (aucun garde « une seule instance »), [db.rs:227-233](../src-tauri/src/db.rs#L227) (pas de délai d'attente SQLite).
- **Explication** : rien n'empêche de lancer l'application deux fois (double clic sur le raccourci alors qu'elle est déjà ouverte). Les deux copies gardent chacune leur état en mémoire et écrivent dans la même base : la dernière qui enregistre l'espace de travail écrase l'autre. Sans `busy_timeout`, une écriture simultanée échoue tout de suite (« database is locked »). Les raccourcis globaux et la surveillance tournent en double.
- **Reproduire** : lancer Grimoire deux fois, déplacer des cartes dans l'une puis dans l'autre.
- **Piste** : `tauri-plugin-single-instance` (licence MIT / Apache, compatible GPL) : le second lancement remet la première fenêtre au premier plan.
- **Certitude** : très probable (lu, non testé).

### 3. Aucune sauvegarde avant une migration de la base

- **Où** : [lib.rs:66](../src-tauri/src/lib.rs#L66) (ouverture + migrations) puis [lib.rs:73](../src-tauri/src/lib.rs#L73) (sauvegardes lancées après) ; [backup.rs:192-204](../src-tauri/src/backup.rs#L192).
- **Explication** : au premier lancement après une mise à jour, les migrations modifient la base **avant** la sauvegarde automatique du jour. Si une migration a un défaut, la dernière copie saine date de la veille au mieux (plus si Grimoire n'a pas été ouvert). Les migrations v8 (tables reconstruites) et v12 (colonne supprimée) montrent que ce ne sont pas toujours de petits changements. Avec la mise à jour en un clic, tous les joueurs y passent en même temps.
- **Piste** : si `user_version` est inférieur à la version du logiciel, copier `donnees.sqlite` (sorte « avant-mise-a-jour ») avant de migrer.
- **Certitude** : certain pour l'ordre des opérations ; le risque dépend de la qualité des migrations à venir.

### 4. Base d'une version plus récente : ouverte sans rien dire

- **Où** : [db.rs:253-262](../src-tauri/src/db.rs#L253), fonction `migrate` (`skip(current as usize)`).
- **Explication** : la restauration d'une sauvegarde refuse une base « plus récente que le logiciel » ([backup.rs:215](../src-tauri/src/backup.rs#L215)), mais le démarrage normal ne vérifie pas. Un joueur de la version portable qui remet un ancien `Grimoire.exe` sur des données récentes obtient des erreurs SQL en cascade (colonne inconnue), sans explication.
- **Test** : une base passée en version 99 est rouverte sans erreur.
- **Certitude** : certain pour l'absence de contrôle ; conséquences probables.

### 5. Commandes sur le fil principal, verrou de la base tenu pendant les sauvegardes

- **Où** : [backup.rs:246-247](../src-tauri/src/backup.rs#L246) et [264-265](../src-tauri/src/backup.rs#L264) (verrou tenu pendant toute la copie), [library.rs:20-23](../src-tauri/src/library.rs#L20), [library.rs:240-241](../src-tauri/src/library.rs#L240) (`game_recap` lit `localconfig.vdf`, plusieurs Mo, dans une commande non asynchrone).
- **Explication** : dans Tauri, une commande écrite sans `async` s'exécute sur le fil principal de l'application. Presque toutes les commandes de Grimoire sont ainsi et attendent le verrou de la base. La sauvegarde automatique part **au démarrage** et tient ce verrou pendant qu'elle copie la base et les jaquettes : les premières commandes de l'interface attendent derrière. Avec une grosse bibliothèque ou un disque lent, le premier lancement de la journée peut paraître figé une ou deux secondes.
- **Piste** : retarder la sauvegarde automatique de 30 s ; marquer les commandes lentes `#[tauri::command(async)]`.
- **Certitude** : probable (dépend de la taille des données ; non mesuré).

### 6. Les succès obtenus pendant que Grimoire était fermé ne sont pas vus au démarrage

- **Où** : [watch.rs:123-131](../src-tauri/src/watch.rs#L123) (`file_changed` : la première date vue sert de référence, sans lecture), démarrage dans [App.svelte:38-68](../src/App.svelte#L38) (aucune actualisation).
- **Explication** : la surveillance ne réagit qu'aux changements survenus **pendant** que Grimoire tourne. Un succès débloqué Grimoire fermé n'apparaît qu'après un clic sur « Actualiser », jeu par jeu, ou au prochain changement du fichier. Le joueur peut croire que l'actualisation automatique ne marche pas.
- **Piste** : au démarrage, relire le cache local des jeux dont le fichier de progression est plus récent que `games.refreshed_at` (lecture locale, sans réseau).
- **Certitude** : très probable (lu ; choix peut-être voulu, mais non écrit dans `docs/choix.md`).

### 7. Fenêtre compacte : tout est rechargé à chaque changement dans la fenêtre principale

- **Où** : [Compact.svelte:256](../src/Compact.svelte#L256) (`data-changed` → `load`), [store.svelte.ts:34-38](../src/lib/blocks/store.svelte.ts#L34) (`loadBlocks` vide d'abord `blocks.byCard`).
- **Explication** : déplacer une carte, cocher un succès ou renommer une carte libre dans la fenêtre principale envoie `data-changed`. La fenêtre compacte vide alors tous ses blocs puis les recharge : la liste clignote, le défilement revient en haut, et un lecteur vidéo ouvert est détruit puis recréé (il repart à sa dernière position, mais se recharge).
- **Piste** : charger d'abord, remplacer ensuite (ne pas passer par l'état vide), et ne recharger que ce qui a changé.
- **Certitude** : probable (lu ; l'interface simulée ne relie pas les deux fenêtres, donc non observé).

### 8. Réseau qui ne répond pas : import ou actualisation de plus d'une minute

- **Où** : [web.rs:15-24](../src-tauri/src/steam/web.rs#L15) (délai de 20 s par requête, pas de délai de connexion), enchaînement dans [library.rs:198-229](../src-tauri/src/library.rs#L198) et [mod.rs:76](../src-tauri/src/steam/mod.rs#L76).
- **Explication** : réseau coupé net, tout échoue vite et l'import local fonctionne. Mais sur un réseau qui « avale » les requêtes (portail d'hôtel, pare-feu), chaque appel attend 20 s et ils se suivent : rareté, nom du magasin, adresses des images, jaquette. Le bouton reste sur « Actualisation… » plus d'une minute alors que les données locales étaient prêtes.
- **Piste** : `connect_timeout` de 5 s, et lancer ces requêtes en parallèle.
- **Certitude** : probable (non testé).

### 9. Recherche « Ajouter un jeu » : le cache de Steam est relu à chaque frappe

- **Où** : [library.rs:52-69](../src-tauri/src/library.rs#L52) et [155](../src-tauri/src/library.rs#L155), [local_cache.rs:38-57](../src-tauri/src/steam/local_cache.rs#L38), [manifests.rs:56-76](../src-tauri/src/steam/manifests.rs#L56).
- **Explication** : chaque recherche relit le dossier `appcache/stats` (984 jeux sur votre PC), ouvre le schéma de chaque jeu non installé pour y trouver son nom, et lit les manifestes d'installation **deux fois** (une dans `list_cached_games`, une dans `local_games`). Sur un SSD ça passe ; sur un disque dur ou avec un antivirus, la recherche peut traîner.
- **Piste** : garder la liste en mémoire une minute.
- **Certitude** : probable (non mesuré).

### 10. Autres risques, plus faibles

- **Sauvegarde abîmée** : [backup.rs:212-221](../src-tauri/src/backup.rs#L212). Avant de restaurer, seule la version de la base est lue. Une sauvegarde partiellement abîmée pourrait remplacer des données saines (l'état d'avant est heureusement gardé en « avant-restauration »). Un `PRAGMA quick_check` avant la restauration fermerait la porte. Probable.
- **Sauvegardes sur le même disque que les données** : `backups/` est dans `%APPDATA%\Grimoire`. Disque mort ou profil Windows perdu = tout perdu. Certain (voir idée 3).
- **Dossiers temporaires de sauvegarde** : [backup.rs:134-135](../src-tauri/src/backup.rs#L134). Un `.…tmp` laissé par un plantage n'est jamais nettoyé (son nom contient l'heure, il ne sera jamais réutilisé). Très probable, peu grave.
- **Position de carte sans limite haute** : [db.rs:797](../src-tauri/src/db.rs#L797) borne `x`, `w`, `h` mais pas `y`. Test : un guide avec `y = 9 000 000 000 000 000` est enregistré tel quel. Effet sur le tableau non observé (hauteur de page démesurée probable).
- **Succès sans ID technique** : [db.rs:279-281](../src-tauri/src/db.rs#L279). Leur repère est leur nom ; un nom qui change (langue, correction de l'éditeur) crée un succès en double et marque l'ancien « retiré », avec ses guides. Rare : les trois sources donnent presque toujours l'ID.
- **Liens avec des astérisques dans un bloc Texte** : [markdown.ts:19-24](../src/lib/markdown.ts#L19). Le gras et l'italique sont appliqués après les liens : une adresse contenant `*…*` reçoit des balises au milieu et ne s'ouvre plus. Pas un problème de sécurité (tout est échappé avant). Très probable, non testé.
- **Plusieurs comptes Steam sur le PC** : [mod.rs:57-61](../src-tauri/src/steam/mod.rs#L57). Seul le compte le plus récent est lu. Voulu et écrit, mais invisible pour le joueur.

---

## 3. Améliorations

1. **Journal d'erreurs** : les `eprintln!` ([watch.rs:90](../src-tauri/src/watch.rs#L90) et [106](../src-tauri/src/watch.rs#L106), [backup.rs:248](../src-tauri/src/backup.rs#L248), [lib.rs:61](../src-tauri/src/lib.rs#L61)) n'apparaissent nulle part dans la version publiée (pas de console). Un simple `grimoire.log` dans le dossier des données, limité en taille, rendrait les rapports de bugs exploitables. Fort intérêt pour peu de travail.
2. **Remise en forme des blocs** au chargement et à l'import (voir bug 4) : c'est aussi ce qui permettra de faire évoluer le format des blocs sans migration.
3. **Fenêtres modales** : passer les quatre fenêtres en `<dialog>` natif (voir bug 6). Menus « ⋯ » ([MoreMenu.svelte:56-63](../src/lib/MoreMenu.svelte#L56)) : flèches Haut / Bas, et retour du focus sur le bouton à la fermeture (aujourd'hui Échap ferme mais le focus est perdu).
4. **Vue Tableau au clavier** : déplacer et redimensionner une carte ne se fait qu'à la souris ([Board.svelte:118-132](../src/lib/Board.svelte#L118)). La vue Liste, elle, marche au clavier, et le menu « Déplacer vers… » aide. À signaler dans l'aide plutôt qu'à refaire.
5. **Liste des succès non virtualisée** : [AchievementList.svelte:184](../src/lib/AchievementList.svelte#L184) crée tous les éléments d'un coup, chacun avec ses menus. Bien pour 50 succès ; à surveiller pour les jeux qui en ont plusieurs centaines (Vampire Survivors : 220, certains dépassent 1 000). À mesurer avant d'agir.
6. **Tests de l'interface** : aucun test côté TypeScript. `filters.ts`, `markdown.ts`, `blocks/ops.ts`, `youtube.ts`, `shortcuts.svelte.ts` sont de la logique pure, faciles à tester (vitest, licence MIT). Le workflow [verifications.yml](../.github/workflows/verifications.yml) ne lance que `npm run check` et `cargo test`.
7. **Workflow de publication** : [publication.yml:55](../.github/workflows/publication.yml#L55) utilise `tauri-apps/tauri-action@v0` (étiquette mobile) et lui confie la clé de signature des mises à jour. Épingler les actions par leur empreinte de commit évite qu'une version modifiée de l'action signe une mise à jour à votre place. Faible probabilité, conséquence maximale.
8. **Progression du téléchargement de la mise à jour** : [update.rs:93](../src-tauri/src/update.rs#L93) ignore l'avancement (`|_, _| {}`). Le bandeau reste sur « Téléchargement… » sans indication ; sur une connexion lente, on ne sait pas si ça avance.
9. **Permissions par fenêtre** : [capabilities/default.json](../src-tauri/capabilities/default.json) donne les mêmes droits aux deux fenêtres. La principale n'a pas besoin d'enregistrer des raccourcis globaux. Détail, la configuration est déjà serrée (CSP stricte, portée des images donnée à l'exécution, aucune permission de fichiers ni de dialogue côté interface).
10. **Messages d'erreur bruts** : beaucoup de commandes renvoient l'erreur SQLite telle quelle ([library.rs:39-41](../src-tauri/src/library.rs#L39)), affichée en anglais au joueur (« Query returned no rows »). À traduire en une phrase simple pour les cas courants.

---

## 4. Idées

Aucune ne touche à ce qui a été abandonné (clé API, guides partagés, vitrine, masquage des jeux finis, confettis) ni à l'ordre prévu (traduction en dernier, Windows seulement).

1. **« Tout actualiser » au démarrage, depuis le cache local seulement** : règle le risque 6 sans réseau ni attente, et rend l'actualisation automatique cohérente (« Grimoire est toujours à jour »).
2. **Compléter le cache local par le profil quand il est vieux** : si le joueur a joué sur un autre PC ou un Steam Deck, le fichier local est en retard. L'ordre des sources resterait le même (cache local d'abord, comme décidé) ; le profil ne ferait qu'**ajouter** des succès obtenus que le cache ignore, jamais en retirer.
3. **Copie des sauvegardes ailleurs** : un bouton « Exporter une sauvegarde… » (un `.zip` vers OneDrive, une clé USB) ou un second dossier au choix. Aujourd'hui, sauvegardes et données meurent ensemble.
4. **« Vérifier mes données »** dans les Réglages : `PRAGMA integrity_check`, compte des images manquantes, bouton « ouvrir le journal ». Rassurant, et utile pour le support.
5. **Choix du compte Steam** quand le PC en a plusieurs (aujourd'hui : le plus récent, sans le dire).
6. **Corbeille pour les cartes libres** : la suppression d'une carte libre est définitive (avec confirmation). Garder la carte quelques jours, ou proposer « Annuler » comme pour un bloc, éviterait une perte sur un clic trop rapide.
7. **Nom anglais des succès pour le bouton « Aide »** : déjà noté dans `docs/choix.md` comme la suite logique ; je confirme que c'est la meilleure piste pour de meilleurs résultats de recherche.

---

## 5. Ce qui est bien fait (à ne pas casser)

- **Steam en lecture seule** : aucune écriture, aucune commande exécutée ; le re-lock ne fait qu'afficher et copier.
- **Import de guide « tout ou rien »** avec savepoint et nettoyage des images copiées ; noms de fichiers d'images jamais repris du guide (pas de sortie du dossier possible).
- **Images** : format reconnu par les premiers octets, pas par l'extension ; SVG refusé.
- **Sauvegardes** : écrites sous un nom temporaire puis renommées, identifiants contrôlés, état d'avant gardé à chaque restauration.
- **Surveillance** : fichier vide = « état inconnu », date retenue seulement après une lecture réussie, abandon après 5 échecs. Ce sont exactement les bons réflexes ; il ne manque que la limite de profondeur du bug 2.
- **Rendu du texte** : tout est échappé avant la mise en forme, seuls les liens `http(s)` sont cliquables, et la CSP bloque le reste.
- **Mises à jour** : signature vérifiée, appelées côté Rust seulement, version portable traitée à part.

## Ordre de correction conseillé

1. Bug 1 (blocs perdus à l'import) : une ligne, et un test.
2. Bug 2 (limite de profondeur) : quelques lignes, et un test.
3. Risque 3 + bug 3 (copie avant migration, message si la base est illisible) : à faire **avant** la prochaine version qui contient une migration.
4. Risque 1 (espace vidé) : garde `loaded` + `appid` passé avec la liste.
5. Bug 4 (remise en forme des blocs), puis bug 5 (DLC), bug 7 (une ligne), bug 6 (fenêtres modales).
6. Risque 2 (une seule instance) et amélioration 1 (journal).
