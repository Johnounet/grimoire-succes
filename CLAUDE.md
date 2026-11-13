# CLAUDE.md

**Grimoire** : logiciel Windows pour préparer et suivre ses succès Steam jusqu'au 100 %. Le cahier des charges d'origine est dans [SPEC.md](SPEC.md), **à consulter seulement si besoin** : une bonne partie a été dépassée par les décisions ci-dessous. Le détail des choix faits et des pièges, sujet par sujet, est dans [docs/choix.md](docs/choix.md) : **lire la section concernée avant de toucher à un sujet**, et la tenir à jour (ce fichier-ci reste court : il est chargé à chaque message).

## Règles impératives

- Ne jamais écrire dans les fichiers de Steam : lecture seule. Ne jamais exécuter de commande console Steam : seulement les afficher.
- Windows 11 seulement (décision de l'utilisateur, 2026-11-12) : Linux non publié ni annoncé, mais le code reste portable (chemins Steam Linux conservés, pas de code propre à Windows sans nécessité).
- Logique Steam isolée dans `src-tauri/src/steam/`, réparable seule si Valve change un format.
- Licence GPL-3.0 : toute dépendance ajoutée doit être compatible.
- **Ne pas lancer l'application depuis la session Claude** (Windows isole alors `%APPDATA%`) : l'utilisateur lance `npm run tauri dev` dans son terminal et teste sous Windows avant de valider.

## Stack et architecture

Tauri 2 (cœur Rust dans `src-tauri/`) + Svelte 5 + TypeScript + Vite + Tailwind 4 (interface dans `src/`).

- SQLite via `rusqlite`, uniquement côté Rust (`db.rs`). L'interface appelle des commandes Tauri (`library.rs`, miroir TS dans `src/lib/library.ts`). Migrations par `PRAGMA user_version` : ne jamais modifier une migration existante, en ajouter une.
- Tout nouveau réglage : l'ajouter à la liste `SETTINGS` de `library.rs` et au type `SettingKey`.
- Toute nouvelle commande Tauri : l'ajouter aussi à `src/dev-mock.ts`.
- Données dans `%APPDATA%\Grimoire` (`data_dir.rs`, pas le dossier de l'identifiant qui contient le pseudo) : `donnees.sqlite`, `images/` (portée du protocole asset donnée à l'exécution, celle de `tauri.conf.json` est vide), `backups/`.
- Les guides référencent `achievements.id`, stable lors des actualisations. Une carte de l'espace se repère par sa `cardKey` (succès positif, carte libre négative), jamais par `achievement_id`.
- Fenêtre compacte : 2ᵉ fenêtre Tauri `compact`, synchronisée par événements (`src/lib/sync.ts`).
- Couleurs : toujours les variables `--color-*` du thème (`text-on-accent` sur un bouton d'accent). Animations JS : toujours via `dur()` / `reduced()` de `motion.svelte.ts`.

## Commandes

```bash
npm install            # dépendances front
npm run tauri icon app-icon.svg   # génère src-tauri/icons/ (requis avant le 1er build)
npm run tauri dev      # lancer l'application (par l'utilisateur)
npm run tauri build    # produire les installeurs
npm run check          # vérification des types Svelte/TS
cd src-tauri && cargo test --lib                                  # tests Rust
cd src-tauri && cargo run --example probe -- <appid> [french|english] [profil]   # diagnostic sans interface
```

**Interface simulée** : `npm run dev`, puis http://localhost:1420 (données factices de `src/dev-mock.ts`, dev seulement) ; `?compact` pour la fenêtre compacte, `?empty` pour une bibliothèque vide. Dans le volet masqué du navigateur, les animations restent figées : les couper avant une capture.

## Pièges (ne pas refaire)

- Tableau (`Board.svelte`, gridstack) : `setupItem` ne doit s'exécuter qu'une fois (`untrack`) ; ne pas utiliser `grid.getCellFromPixel` (case calculée par `cellAt`) ni `grid.compact()`.
- svelte-dnd-action : pas de `dragHandle` pour des zones imbriquées ; ne pas déstructurer l'élément dans le `{#each}` d'une zone.
- Ne pas remplacer un objet `$state` lu par un `$effect` (ex. `ui.relock`) : modifier ses champs.
- Thème « selon Windows » : rendre la fenêtre au système (`setTheme(null)`), jamais la forcer.
- `dragDropEnabled: false` dans `tauri.conf.json` (glisser HTML natif depuis le panneau).
- Sources Steam : `?xml=1` des profils ne marche plus ; langue des pages par le cookie `Steam_Language` ; nom du jeu pris sur le magasin (le schéma local peut contenir un nom de code). Détails : docs/choix.md, « Sources Steam ».

## Décisions de l'utilisateur à respecter

- Vue Tableau (grille libre) à la place des colonnes du SPEC ; vue Liste conservée.
- Abandonnés : clé API Steam, bibliothèque de guides partagés (l'export / import reste un simple fichier), vitrine des trophées et masquage des jeux à 100 %, confettis. Un jeu fini se retire de l'application ; la célébration du 100 % reste.
- Au démarrage : dernier jeu consulté ; page d'accueil seulement pour une bibliothèque vide, sinon par son bouton.
- Traduction français / anglais en tout dernier.
- Nom : **Grimoire**, identifiant `io.github.johnounet.grimoire` (à ne changer qu'avant la première version publiée). `GITHUB_URL` de `AboutDialog.svelte` à renseigner quand le dépôt existera. Mention « vibe-codé avec Claude » à garder visible.

## Avancement

Étapes 1 à 7 du SPEC validées sous Windows (2026-11-12). **Étape 8 (finitions) en cours**, validés sous Windows : 100 % et célébration, animations, cartes libres, refonte du mode compact, actualisation automatique, export / import de guides, thèmes. Faits aussi : progression Steam des succès, sauvegardes, dossier `Grimoire`.

- [ ] Page d'accueil, réouverture du dernier jeu, suppression de la vitrine (2026-11-13) : à tester sous Windows
- [ ] Traduction (en dernier)

## Index de docs/choix.md

Dossier des données · Page d'accueil · Thèmes · Export / import de guides · Sauvegardes · Progression Steam des succès · Actualisation automatique · Mode compact · Cartes libres · Animations et son · 100 % et célébration · Ordre des jeux du panneau · Étiquettes, re-lock, raccourcis, synchronisation · Tableau, panneau des jeux, À propos · Blocs · Glisser-déposer · Sources Steam · Nom · Historique des étapes
