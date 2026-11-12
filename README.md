# Chasse aux succès

*(nom provisoire)*

Logiciel de bureau gratuit et open source pour les chasseurs de succès Steam : la liste des succès d'un jeu et, sous chacun, vos propres guides (vidéos YouTube, images, notes, listes, compteurs, chronos), sauvegardés automatiquement en local.

- Windows 11 (Linux non pris en charge : le code est portable mais n'a jamais été testé, à compiler soi-même à ses risques)
- Aucune clé API Steam requise
- Ne modifie jamais vos succès Steam : lecture seule

> Projet en cours de développement (étape 7 sur 9). Le cahier des charges est dans [SPEC.md](SPEC.md).

## Développement

### Prérequis

- [Node.js](https://nodejs.org/) 20 ou plus récent
- [Rust](https://rustup.rs/) (stable)
- Dépendances système de Tauri 2 : voir <https://v2.tauri.app/start/prerequisites/>
  - **Windows** : Microsoft C++ Build Tools (charge de travail « Développement Desktop en C++ ») ; WebView2 est déjà présent sur Windows 11.
  - **Linux (non testé)**, Debian/Ubuntu :
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```
  - **Linux (non testé)**, Fedora :
    ```bash
    sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file libappindicator-gtk3-devel librsvg2-devel libxdo-devel
    sudo dnf group install "c-development"
    ```

### Premier lancement

```bash
npm install
npm run tauri icon app-icon.svg   # génère les icônes dans src-tauri/icons/
npm run tauri dev
```

### Autres commandes

| Commande | Rôle |
| --- | --- |
| `npm run tauri dev` | Lance l'application avec rechargement à chaud |
| `npm run tauri build` | Produit les installeurs dans `src-tauri/target/release/bundle/` |
| `npm run check` | Vérifie les types Svelte / TypeScript |

## Structure

```
src/          Interface (Svelte 5 + TypeScript)
src-tauri/    Cœur de l'application (Rust, Tauri 2)
SPEC.md       Cahier des charges
```

## Licence

[GPL-3.0](LICENSE) : vous pouvez utiliser, modifier et redistribuer ce logiciel, à condition que les copies et versions modifiées restent sous la même licence.

Ce projet n'est ni affilié à Valve ni approuvé par Valve. Steam est une marque de Valve Corporation.
