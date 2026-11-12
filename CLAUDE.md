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

Tauri 2 (cœur Rust dans `src-tauri/`) + Svelte 5 + TypeScript + Vite (interface dans `src/`). Tailwind, SQLite, i18n, etc. arrivent aux étapes qui en ont besoin.

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

Diagnostic sans interface : `cd src-tauri && cargo run --example probe -- <appid> [french|english] [profil]`.

## Sources Steam : constats de l'étape 2 (le SPEC date d'avant)

- **`?xml=1` des profils ne marche plus** : renvoie une page HTML.
- Les pages `steamcommunity.com/stats/<appid>/achievements/` et `…/profiles/<id>/stats/<appid>/achievements/` sont rendues côté serveur avec un JSON embarqué (`<script id="valve-ssr-data">` → `renderContext.queryData` → requêtes `["achievements", appid, "schema_v2" | "globalpercentages" | "user_achievements"]`). La page publique fournit donc **l'ID technique, les descriptions des succès cachés, la rareté et le DLC de chaque succès** ; le profil y ajoute l'état et la date. Lecture dans `src-tauri/src/steam/web.rs`.
- La langue des pages se choisit par le cookie `Steam_Language=<langue>` (le paramètre `?l=` est ignoré).
- Profil public + jeu non possédé : la progression arrive quand même, tout à « non obtenu ». Ne pas confondre avec « possédé, rien obtenu ».
- Cache local : bit débloqué = `(stat, bit)` du schéma ↔ champ de bits `data` de la progression ; dates dans `AchievementTimes`.

## Nom provisoire

Le nom du logiciel est une question ouverte du SPEC. Nom de travail : **Chasse aux succès** (`chasse-succes`, identifiant `io.github.chassesucces.app`). À remplacer partout (package.json, Cargo.toml, tauri.conf.json, README) une fois choisi.
