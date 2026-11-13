# CLAUDE.md

Le cahier des charges complet du projet est dans [SPEC.md](SPEC.md). **Le lire avant toute tâche** : vision, fonctionnalités, sources Steam, modèle de données, choix techniques et étapes de développement y sont décrits.

On avance une étape à la fois (section « Étapes de développement avec Claude Code » du SPEC). Ne pas anticiper les étapes suivantes au-delà de ce que l'architecture exige.

## Règles impératives

- Ne jamais écrire dans les fichiers de Steam : lecture seule.
- Ne jamais exécuter de commande console Steam : seulement les afficher.
- Tester chaque étape sous Windows 11 avant de continuer. **Décision de l'utilisateur (2026-11-12)** : Windows seulement, Linux non pris en charge (l'utilisateur ne peut pas le tester). Le code reste portable (chemins Steam Linux conservés, pas de code propre à Windows sans nécessité), mais aucune version Linux n'est publiée ni annoncée.
- Garder la logique Steam isolée dans un module Rust dédié, réparable seul si Valve change un format.
- Licence GPL-3.0 : toute dépendance ajoutée doit être compatible.

## Stack

Tauri 2 (cœur Rust dans `src-tauri/`) + Svelte 5 + TypeScript + Vite + Tailwind 4 (interface dans `src/`).

- **Base de données** : SQLite via `rusqlite`, uniquement côté Rust (`src-tauri/src/db.rs`). L'interface ne touche jamais la base : elle appelle des commandes Tauri (`src-tauri/src/library.rs`). Migrations par `PRAGMA user_version` : ne jamais modifier une migration existante, en ajouter une.
- **Données** : dossier `app_data_dir` de Tauri (`%APPDATA%\io.github.chassesucces.app`, `~/.local/share/io.github.chassesucces.app`) : `donnees.sqlite` + `images/` (jaquettes servies par le protocole asset).
- Les guides (étapes suivantes) devront référencer `achievements.id`, stable lors des actualisations.

## Commandes

```bash
npm install            # dépendances front
npm run tauri icon app-icon.svg   # génère src-tauri/icons/ (requis avant le 1er build)
npm run tauri dev      # lancer l'application en développement
npm run tauri build    # produire les installeurs
npm run check          # vérification des types Svelte/TS
```

## Avancement

- [x] Étape 1 — Squelette : fenêtre validée sous Windows 11 (2026-11-12)
- [x] Étape 2 — Prototype des sources Steam : validé sous Windows sur Portal 2, Clair Obscur, Vampire Survivors, Baldur's Gate 3, Hades, Balatro (2026-11-12)

- [x] Étape 3 — Base de données et bibliothèque (rusqlite côté Rust, pas `tauri-plugin-sql`) : persistance validée sous Windows (2026-11-12)
- [x] Étape 4 — Liste des succès : validée sous Windows (2026-11-12). Tri « ordre personnel » et filtre par étiquette reportés aux étapes 5 (glisser-déposer) et 7 (étiquettes)

- [x] Étape 5 — Espace de travail : validé sous Windows (2026-11-12). **Décision de l'utilisateur (2026-11-12)** : la vue Colonnes du SPEC est remplacée par une vue **Tableau** (grille aimantée de 12 colonnes, cartes déplaçables et redimensionnables librement, sans chevauchement). Le statut à faire / en cours / fait reste un menu sur chaque carte. Vue Liste conservée

- [x] Étape 6 — Blocs : validé sous Windows (2026-11-12), YouTube compris
- [x] Vue Tableau (remplace les colonnes) : validée par l'utilisateur sous Windows, dépôt depuis le panneau compris (2026-11-12)

- [x] Étape 7 — Étiquettes, re-lock, raccourcis globaux, mode compact : testée par l'utilisateur sous Windows (2026-11-12), premiers retours corrigés (bloc visé par défaut par les raccourcis, cartes qui remontent)

**Étape 8 en cours** (finitions). **Décision de l'utilisateur (2026-11-12)** : la traduction français / anglais se fera tout à la fin.
- [x] 100 % et vitrine : validé par l'utilisateur sous Windows (2026-11-12)
- [x] Animations (réglage, transitions, carillon à la coche) : validé par l'utilisateur (2026-11-12)
- [x] Cartes libres (demande de l'utilisateur) : validées par l'utilisateur sous Windows (2026-11-13)
- [x] Refonte du mode compact (demande de l'utilisateur) : vidéos, sélection Haut/Bas, actions Gauche/Droite ; validée en jeu sous Windows (2026-11-13), vidéo lancée par raccourci avec le son
- [x] Actualisation automatique (demande de l'utilisateur) : validée en jeu sous Windows (2026-11-13)
- [x] Export / import de guides : validé par l'utilisateur sous Windows (2026-11-13)
- [ ] Thèmes. **Décision de l'utilisateur (2026-11-13)** : la clé API Steam optionnelle du SPEC est abandonnée (le cache local et le profil public suffisent). **Décision de l'utilisateur (2026-11-13)** : la « bibliothèque de guides partagés » (V2 du SPEC) est abandonnée ; l'export / import reste un simple fichier (sauvegarde, envoi à un ami)
- [ ] Traduction (en dernier)

**Export / import de guides, choix faits** : `src-tauri/src/guides.rs`. Un fichier `.json` unique (format `chasse-succes-guide`, `version` 1), images des blocs incluses en base64. Contient les cartes (succès repéré par `api_name`, sinon nom ; cartes libres par titre), leur place dans l'espace, les étiquettes (nom + couleur) et les blocs ; **jamais** la progression Steam, les coches, le compte, le temps de jeu ni les réglages. Case « Garder l'état des compteurs, listes et chronos » (sauvegarde) décochée par défaut : guide vierge (compteurs à 0, listes décochées, chronos sans temps ni record, cartes « à faire », pas d'épingle). Import = fusion : jeu ajouté depuis Steam s'il manque, blocs ajoutés après les existants, ce qui existe à l'identique n'est pas recréé (réimporter ne double rien ; une image se compare par sa légende), disposition du Tableau reprise seulement si l'espace est vide, succès introuvable → carte libre « Succès introuvable : … ». Fenêtres Enregistrer / Ouvrir de Windows ouvertes **côté Rust** (`tauri-plugin-dialog`, aucune permission JS). Interface : menu « ⋯ » du jeu (Exporter / Importer), `ExportDialog.svelte`, bouton « Exporter le guide d'abord… » à la suppression d'un jeu, lien « Importer un guide… » dans « Ajouter un jeu ». Après un import, la vue du jeu est recréée (`viewVersion` dans `App.svelte`).

**Actualisation automatique, choix faits** : constat de l'utilisateur (Cookie Clicker, 2026-11-13) : Steam réécrit `UserGameStats_<compte>_<appid>.bin` dès le déblocage, jeu ouvert. `src-tauri/src/watch.rs` : fil de fond lancé au démarrage, toutes les 2 s regarde la date de modification de ce fichier pour chaque jeu de la bibliothèque (lecture seule ; Steam recherché toutes les 60 s s'il est introuvable). Fichier changé → relecture du **seul cache local** (pas de réseau, la rareté en base est gardée par `save_game`) ; rien n'est écrit si l'état des succès dans le fichier n'a pas changé (statistiques seules). Si l'état en base change (coches manuelles respectées) : événement Tauri `steam-progress` `{ game, unlocked }` vers les deux fenêtres. Fenêtre principale : `upsert` (liste, bannière, célébration du 100 %) et notification « 🏆 Succès débloqué » (`showToast(…, { untilSeen: true })` : reste affichée tant que la fenêtre n'est pas au premier plan, puis 10 s) ; fenêtre compacte : coche et message (10 s). Un seul carillon : la fenêtre compacte si elle est ouverte (`shortcuts.live`), sinon la principale.

**Mode compact, refonte (2026-11-13), choix faits** :
- **Éléments** : compteurs, chronos, vidéos et listes des cartes épinglées, dans l'ordre d'affichage. Haut / Bas changent la sélection (boucle, cadre bleu, clic = sélection), Gauche / Droite agissent : compteur −1 / +1 ; chrono remis à zéro **à l'arrêt seulement** / lancé-arrêté ; vidéo −10 s / lancée, lecture-pause ; liste : décoche le dernier coché / coche le suivant. Les coches de succès ne sont pas des éléments (pas de succès obtenu d'une fausse touche).
- **Raccourcis enregistrés par la fenêtre compacte elle-même**, à l'ouverture, et libérés à la fermeture (bouton ✕ et `onCloseRequested`) : hors mode compact, les touches restent libres. La fenêtre principale ne fait que les régler (`shortcuts-changed`) et affiche l'état renvoyé (`shortcuts-status` : actifs ou non, refus du système).
- **Vidéos** : `CompactVideo.svelte`, une ligne puis le lecteur à l'ouverture (lecture automatique), un seul lecteur ouvert à la fois. Lecteur YouTube commun aux deux fenêtres : `src/lib/blocks/player.svelte.ts` (postMessage). Quand une vidéo démarre dans une fenêtre, la même vidéo se met en pause dans l'autre (`video-playing`). Lancement par raccourci sans clic préalable vérifié sous Windows : la vidéo démarre avec le son.
- Les listes envoient aussi `block-changed` (affichées dans les deux fenêtres).
- Interface simulée de la fenêtre compacte : http://localhost:1420/?compact, avec une carte de démonstration (un élément de chaque sorte) ; `window.compactRun("select_next" | "select_prev" | "act_left" | "act_right")` dans la console simule les raccourcis (dev seulement).

**Cartes libres, choix faits** (blocs sans succès, choisis plutôt que des blocs isolés dans le Tableau) :
- Table `free_cards` (migration v8 : `id`, `appid`, `title`). `workspace_items` et `blocks` reconstruites : chaque ligne a soit `achievement_id`, soit `card_id` (contrainte CHECK), les `id` des blocs sont conservés. Côté Rust, `db::Owner` (succès ou carte) pour créer et réordonner les blocs ; commandes `free_cards_list`, `free_card_create`, `free_card_rename`, `free_card_delete`.
- Interface : une carte est repérée par sa **`cardKey`** (`library.ts`) : id du succès (positif) ou opposé de l'id de la carte libre (négatif). Blocs groupés par `cardKey` (`blocks.byCard`), tableau, liste, raccourcis et mode compact l'utilisent ; `ownerOf(key)` redonne la forme attendue par Rust. Ne plus utiliser `achievement_id` comme identifiant de carte.
- Une carte libre n'existe que dans l'espace de travail : la retirer la **supprime** avec ses blocs (confirmation dans la carte si elle a des blocs, pas de taille mémorisée dans `card_sizes`). Si elle manque dans `workspace_items`, `list_workspace` la remet à la fin plutôt que de la perdre.
- Titre modifiable directement dans l'en-tête (focus à la création) ; pas de coche, description, étiquettes ni re-lock ; statut, épingle au mode compact et tous les types de blocs conservés. Bouton « + Carte libre » dans la barre de l'espace de travail.

**Animations, choix faits** :
- Module `src/lib/motion.svelte.ts` : réglage `animations` (`system` = selon Windows, par défaut / `on` / `off`), section « Interface » des Réglages, synchronisé avec la fenêtre compacte (`motion-changed`). Animations réduites → attribut `data-reduced-motion` sur `<html>` (coupe tout le CSS, `app.css`) **et** `dur()` qui renvoie 0 pour ce qui est lancé en JS (transitions Svelte, `flip`, svelte-dnd-action, gridstack via `setAnimation`). Toute nouvelle animation JS doit passer par `dur()` / `reduced()`, jamais `matchMedia` directement.
- Durées communes `SPEED` (120 / 180 / 300 ms). Transitions `backdrop` et `pop` (fenêtres modales, menus « ⋯ », étiquettes, notification, menu d'ajout de bloc en entrée seulement car il est dans le flux de la carte). `<dialog>` natif (ajout de jeu) : classe CSS `dialog-pop`.
- `appear` : changement de jeu et vitrine. `arrive` : une carte ne s'anime à l'entrée que si son succès vient d'être ajouté (`arrivals`, rempli par `addToWorkspace`), pas au chargement. `leave` : animation de sortie avant le retrait (`data-card` sur la carte), limitée dans le temps pour ne jamais retarder le retrait. `appear`/`arrive` lisent le réglage avec `untrack` (sinon l'attachement se rejoue).
- Son : `src/lib/sound.svelte.ts`, réglage `check_sound` (désactivé par défaut), carillon de deux notes synthétisées à la coche (pas au décochage), fenêtre principale et compacte (`sound-changed`). Le bip des raccourcis y est aussi.

**100 % et vitrine, choix faits** :
- `games.completed_at` tenu à jour par `update_completion` (`db.rs`) après chaque import, coche ou reset : posé quand tous les succès non retirés sont obtenus (coche manuelle → maintenant ; actualisation → date du dernier succès Steam), effacé si le jeu n'est plus à 100 %. Migration v7 : colonne `celebrated` et date des jeux déjà terminés.
- Célébration (`Celebration.svelte`) **une seule fois par jeu** : déclenchée dans `App.svelte` (`upsert`) quand un jeu déjà présent passe à 100 % et que `celebrated` est faux, puis `game_mark_celebrated`. Un jeu importé déjà terminé rejoint la vitrine sans célébration. Rejouable depuis la vitrine (🎉) et le badge « 🏆 100 % » de la bannière.
- Récap (`game_recap`) : date du 100 %, temps de jeu relu dans `userdata/<compte>/config/localconfig.vdf` (`steam/playtime.rs`, aussi lu à chaque import), succès obtenu le plus rare.
- Confettis : canvas maison (`src/lib/confetti.ts`), couleurs extraites de la jaquette (repli doré si l'image est illisible) ; aucun confetti si « réduire les animations » est actif dans Windows.
- Version dorée : classe `.gold-frame` (`app.css`) ; vitrine (`Showcase.svelte`) ouverte depuis le haut du panneau des jeux, triée par date du 100 %.

**Étape 7, choix faits** :
- **Étiquettes** : tables `tags` / `achievement_tags` (migration v5, 6 étiquettes proposées), communes à tous les jeux. Store `src/lib/tags.svelte.ts`, sélecteur `TagPicker.svelte` (cartes et liste), gestion dans `SettingsDialog.svelte`, filtre dans la liste.
- **Re-lock** : menu « ⋯ » d'un succès (carte ou liste) ou du jeu (bannière) → `RelockDialog.svelte`. Seules actions vers Steam : copier la commande, ouvrir `steam://open/console` (autorisé dans `capabilities/default.json`). « C'est fait » décoche dans le logiciel (`achievements_lock_all` pour le reset complet).
- **Bloc actif ◎** : supprimé lors de la refonte du mode compact (2026-11-13), remplacé par la sélection de la fenêtre compacte.
- **Raccourcis globaux** : `tauri-plugin-global-shortcut`, voir « Mode compact, refonte » ci-dessous. Touches au format `KeyboardEvent.code` (défaut Ctrl+Alt+flèches), réglage `shortcuts` en JSON (anciennes actions `counter_inc`… relues sur les nouvelles). Logique compteur/chrono/liste partagée dans `src/lib/blocks/ops.ts`.
- **Mode compact** : 2ᵉ fenêtre Tauri `compact` (même page, `main.ts` choisit `Compact.svelte` selon le nom de la fenêtre), sans bordure, transparente, toujours au premier plan. Montre les cartes épinglées (menu « ⋯ » de la carte, colonne `workspace_items.pinned`). Fermer la fenêtre principale quitte l'application.
- **Synchronisation entre fenêtres** : événements Tauri (`src/lib/sync.ts`) : `block-changed`, `data-changed`, `achievement-changed`, `shortcuts-changed`, `shortcuts-status`, `video-playing`, `compact-game`… Chaque fenêtre ignore ses propres messages. Ne pas remplacer un objet `$state` lu par un `$effect` (ex. `ui.relock`) : modifier ses champs.
- Interface simulée de la fenêtre compacte : http://localhost:1420/?compact (données simulées propres à l'onglet, voir « Mode compact, refonte »).

**Retours sur l'étape 7 (2026-11-12)** : panneau des jeux repliable (réglage `sidebar_open`, replié = jaquettes seules) ; fenêtre « À propos » (`AboutDialog.svelte`, bouton à côté de Réglages : version, dépôt GitHub à renseigner dans `GITHUB_URL`, contact Discord, mention « vibe-codé avec Claude » à garder visible) ; dans le Tableau, une carte s'agrandit en hauteur (jamais ne rapetisse) quand son contenu grandit **à la suite d'une action du joueur dans la carte** (`watchGrowth` dans `Board.svelte` : ResizeObserver sur `[data-card-content]`, ignoré au chargement et pendant un redimensionnement, pour qu'une carte réduite exprès le reste) ; la vidéo émet aussi `FIT_CARD_EVENT`. Le menu « Ajouter un bloc » est donc dans le flux de la carte, pas en surimpression ; il émet `CARD_MENU_EVENT` à l'ouverture et à la fermeture : fermé sans ajout, la carte reprend sa taille d'avant et les cartes poussées remontent (`menuToggled`) ; après un ajout, `FIT_CARD_EVENT` l'ajuste au nouveau bloc. Dernière taille de chaque carte gardée dans `card_sizes` (migration v6, mise à jour par `save_workspace`) : une carte remise dans l'espace la retrouve, puis grandit si ses blocs ne tiennent pas (`fresh`). Tout nouveau réglage doit aussi être ajouté à la liste `SETTINGS` de `library.rs`.

**Blocs** : table `blocks` (migration v3) rattachée au succès, pas à l'espace de travail. État et enregistrement différé (400 ms) dans `src/lib/blocks/store.svelte.ts` ; `flushBlocks()` au changement de jeu et à la fermeture (`onCloseRequested`, d'où la permission `core:window:allow-destroy`). Images dans `images/blocks/`, envoyées en binaire brut (`image_save`), orphelines supprimées au démarrage. Réordonnancement des blocs en glisser HTML natif : **ne pas utiliser `dragHandle` de svelte-dnd-action pour des zones imbriquées** (état de poignée global, la carte parente se mettrait à glisser aussi).

**Interface simulée** : `npm run dev` puis ouvrir http://localhost:1420 dans un navigateur. Hors de Tauri et en dev seulement, `src/dev-mock.ts` remplace le cœur Rust par des données factices (absent des versions compilées). Penser à y ajouter chaque nouvelle commande Tauri.

**Glisser-déposer** :
- Vue Tableau : `gridstack` (`src/lib/Board.svelte`), position/taille en base (`workspace_items.grid_*`, migration v4). Svelte rend les cartes, gridstack les positionne : la mise en place d'une carte (`setupItem`) ne doit s'exécuter **qu'une fois** (`untrack`), sinon chaque enregistrement la remet à son ancienne place. Pour tester sans souris réelle : événements souris simulés progressifs sur `document` (un `left_click_drag` instantané ne suffit pas).
- Tableau libre (`float`) : seules les cartes sous une carte retirée remontent (`closeGap`, après `tick()` sinon la carte retirée revient en base), et toutes avec « tout remonter » (remontée verticale maison, pas `grid.compact()` qui déplace les cartes de côté).
- Vue Liste : `svelte-dnd-action` (`dragHandleZone`, `useCursorForDetection: true`).
- Arrivée depuis le panneau : glisser HTML natif, d'où `dragDropEnabled: false` dans `tauri.conf.json`. Dans le Tableau, la case est calculée à la main (`cellAt`) : **ne pas utiliser `grid.getCellFromPixel`**, qui compare les coordonnées écran à `offsetLeft/offsetTop` et décale la carte. Aperçu de la place d'arrivée, toujours la place libre la plus proche (aucune carte n'est poussée) ; les cartes ignorent la souris pendant ce glisser (sinon les iframes YouTube avalent l'événement).

**Ne pas lancer l'application depuis la session Claude** sur ce PC (Windows isole alors `%APPDATA%` dans une copie privée) : c'est l'utilisateur qui lance `npm run tauri dev` dans son terminal.

Diagnostic sans interface : `cd src-tauri && cargo run --example probe -- <appid> [french|english] [profil]`.

## Sources Steam : constats de l'étape 2 (le SPEC date d'avant)

- **`?xml=1` des profils ne marche plus** : renvoie une page HTML.
- Les pages `steamcommunity.com/stats/<appid>/achievements/` et `…/profiles/<id>/stats/<appid>/achievements/` sont rendues côté serveur avec un JSON embarqué (`<script id="valve-ssr-data">` → `renderContext.queryData` → requêtes `["achievements", appid, "schema_v2" | "globalpercentages" | "user_achievements"]`). La page publique fournit donc **l'ID technique, les descriptions des succès cachés, la rareté et le DLC de chaque succès** ; le profil y ajoute l'état et la date. Lecture dans `src-tauri/src/steam/web.rs`.
- La langue des pages se choisit par le cookie `Steam_Language=<langue>` (le paramètre `?l=` est ignoré).
- Profil public + jeu non possédé : la progression arrive quand même, tout à « non obtenu ». Ne pas confondre avec « possédé, rien obtenu ».
- Cache local : bit débloqué = `(stat, bit)` du schéma ↔ champ de bits `data` de la progression ; dates dans `AchievementTimes`.

## Nom provisoire

Le nom du logiciel est une question ouverte du SPEC. Nom de travail : **Chasse aux succès** (`chasse-succes`, identifiant `io.github.chassesucces.app`). À remplacer partout (package.json, Cargo.toml, tauri.conf.json, README) une fois choisi.
