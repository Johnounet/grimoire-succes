# CLAUDE.md

Le cahier des charges complet du projet est dans [SPEC.md](SPEC.md). **Le lire avant toute tâche** : vision, fonctionnalités, sources Steam, modèle de données, choix techniques et étapes de développement y sont décrits.

On avance une étape à la fois (section « Étapes de développement avec Claude Code » du SPEC). Ne pas anticiper les étapes suivantes au-delà de ce que l'architecture exige.

## Règles impératives

- Ne jamais écrire dans les fichiers de Steam : lecture seule.
- Ne jamais exécuter de commande console Steam : seulement les afficher.
- Tester chaque étape sous Windows 11 et Linux avant de continuer.
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

- [x] Étape 1 — Squelette : fenêtre validée sous Windows 11 (2026-11-12) ; reste à valider sous Linux
- [x] Étape 2 — Prototype des sources Steam : validé sous Windows sur Portal 2, Clair Obscur, Vampire Survivors, Baldur's Gate 3, Hades, Balatro (2026-11-12) ; reste Linux

- [x] Étape 3 — Base de données et bibliothèque (rusqlite côté Rust, pas `tauri-plugin-sql`) : persistance validée sous Windows (2026-11-12) ; reste Linux
- [x] Étape 4 — Liste des succès : validée sous Windows (2026-11-12) ; reste Linux. Tri « ordre personnel » et filtre par étiquette reportés aux étapes 5 (glisser-déposer) et 7 (étiquettes)

- [x] Étape 5 — Espace de travail : validé sous Windows (2026-11-12) ; reste Linux. **Décision de l'utilisateur (2026-11-12)** : la vue Colonnes du SPEC est remplacée par une vue **Tableau** (grille aimantée de 12 colonnes, cartes déplaçables et redimensionnables librement, sans chevauchement). Le statut à faire / en cours / fait reste un menu sur chaque carte. Vue Liste conservée

- [x] Étape 6 — Blocs : validé sous Windows (2026-11-12), YouTube compris ; reste Linux (lecture vidéo dépendante de GStreamer)
- [x] Vue Tableau (remplace les colonnes) : validée par l'utilisateur sous Windows, dépôt depuis le panneau compris (2026-11-12) ; reste Linux

**Prochaine étape** : étape 7 du SPEC (étiquettes, re-lock, raccourcis globaux, mode compact).

**Blocs** : table `blocks` (migration v3) rattachée au succès, pas à l'espace de travail. État et enregistrement différé (400 ms) dans `src/lib/blocks/store.svelte.ts` ; `flushBlocks()` au changement de jeu et à la fermeture (`onCloseRequested`, d'où la permission `core:window:allow-destroy`). Images dans `images/blocks/`, envoyées en binaire brut (`image_save`), orphelines supprimées au démarrage. Réordonnancement des blocs en glisser HTML natif : **ne pas utiliser `dragHandle` de svelte-dnd-action pour des zones imbriquées** (état de poignée global, la carte parente se mettrait à glisser aussi).

**Interface simulée** : `npm run dev` puis ouvrir http://localhost:1420 dans un navigateur. Hors de Tauri et en dev seulement, `src/dev-mock.ts` remplace le cœur Rust par des données factices (absent des versions compilées). Penser à y ajouter chaque nouvelle commande Tauri.

**Glisser-déposer** :
- Vue Tableau : `gridstack` (`src/lib/Board.svelte`), position/taille en base (`workspace_items.grid_*`, migration v4). Svelte rend les cartes, gridstack les positionne : la mise en place d'une carte (`setupItem`) ne doit s'exécuter **qu'une fois** (`untrack`), sinon chaque enregistrement la remet à son ancienne place. Pour tester sans souris réelle : événements souris simulés progressifs sur `document` (un `left_click_drag` instantané ne suffit pas).
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
