# Publier une version

Deux téléchargements par version, produits par GitHub Actions (`.github/workflows/publication.yml`) :

- **Installeur** `Grimoire_X.Y.Z_x64-setup.exe` (NSIS, pour l'utilisateur seul, sans droits administrateur). Les copies installées cherchent les nouvelles versions au démarrage et s'installent en un clic.
- **Version portable** `Grimoire_X.Y.Z_x64_portable.zip` : `Grimoire.exe` + `portable.txt`. Données dans `donnees\` à côté de l'exe ; une nouvelle version est annoncée avec un lien de téléchargement (pas d'installation automatique).

## Une seule fois, avant la première version

1. ~~Créer le dépôt GitHub~~ (fait le 2026-11-13) `Johnounet/grimoire-succes` (nom attendu par `plugins.updater.endpoints` de `src-tauri/tauri.conf.json` et `RELEASES_PAGE` de `src-tauri/src/update.rs` : les changer ensemble si le nom diffère). Adresse aussi dans `GITHUB_URL` de `src/lib/AboutDialog.svelte` et dans `src-tauri/capabilities/default.json`.
2. **Créer la clé de signature des mises à jour**, dans son propre terminal PowerShell, depuis le dossier du projet :
   ```powershell
   mkdir "$env:USERPROFILE\.tauri"
   npm run tauri signer generate -- -w "$env:USERPROFILE\.tauri\grimoire.key"
   ```
   Choisir un mot de passe. **Garder la clé privée et le mot de passe en lieu sûr (gestionnaire de mots de passe)** : sans eux, impossible de publier une mise à jour que les copies installées acceptent ; il faudrait que chacun réinstalle à la main.
3. Copier le contenu de la **clé publique** (`grimoire.key.pub`) dans `plugins.updater.pubkey` de `src-tauri/tauri.conf.json`. Tant qu'elle est vide, l'application annonce les nouvelles versions mais ne propose pas de les installer.
4. Dans le dépôt GitHub, *Settings → Secrets and variables → Actions*, créer deux secrets : `TAURI_SIGNING_PRIVATE_KEY` (contenu du fichier `grimoire.key`) et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

## À chaque version

1. Mettre le même numéro dans `package.json`, `src-tauri/Cargo.toml` et `src-tauri/tauri.conf.json` (la publication échoue sinon).
2. Valider, puis créer et envoyer l'étiquette :
   ```bash
   git tag v0.2.0
   git push origin main --tags
   ```
3. Attendre la fin de « Publication » (onglet *Actions*), relire le **brouillon** de version créé (notes, 3 fichiers + `latest.json`), puis le publier. Les copies installées ne voient la version qu'une fois le brouillon publié.

## Bon à savoir

- Une compilation locale (`npm run tauri build`) produit l'installeur sans fichiers de mise à jour : `createUpdaterArtifacts` n'est activé que par la publication, qui a la clé.
- Tester la version portable en local : copier `src-tauri/target/release/grimoire.exe` dans un dossier avec `packaging/portable.txt`, puis le lancer depuis son propre terminal ou l'Explorateur.
- Installeur non signé : Windows SmartScreen affiche un avertissement (« Informations complémentaires » → « Exécuter quand même »). Piste pour l'éviter : signature gratuite pour les projets libres avec SignPath.
