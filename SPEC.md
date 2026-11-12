# Cahier des charges — Outil de chasse aux succès Steam

Version du 12 novembre 2026

## Vision

Un logiciel de bureau gratuit et open source qui réunit sur un seul écran tout ce dont un chasseur de succès Steam a besoin : la liste des succès d'un jeu, et sous chacun, ses propres guides (vidéos, images, notes, listes, compteurs, chronos).

**Problème résolu** : aujourd'hui, le joueur jongle entre un bloc-notes, plusieurs onglets YouTube et des sites de guides. L'outil remplace ce bazar par un espace de travail par jeu, sauvegardé automatiquement.

**Public** : les chasseurs de succès Steam, du joueur occasionnel qui vise un 100 % au complétionniste.

**Principes non négociables**

- Gratuit et open source (licence GPL-3.0, pour que toutes les copies restent open source).
- Fonctionne sous Windows 11. *Décision du 2026-11-12 : Linux (et Steam Deck) n'est pas pris en charge, faute de pouvoir le tester ; le code reste portable pour qu'un contributeur puisse le reprendre.*
- Aucune clé API Steam requise. La clé reste une option pour synchroniser automatiquement les succès débloqués.
- Simple : un jeu s'importe en tapant son nom, son AppID ou en collant le lien de sa page Steam.
- Tout est enregistré en local et retrouvé tel quel au redémarrage.
- Le logiciel ne modifie jamais les succès Steam. Il affiche seulement les commandes, que l'utilisateur lance lui-même.
- Belle interface, agréable à parcourir, avec un 100 % gratifiant.

## Parcours utilisateur

De l'ouverture au premier guide, il faut moins d'une minute et aucun réglage.

1. **Premier lancement** : le logiciel cherche une installation Steam (chemins standards Windows et Linux, Flatpak compris). S'il en trouve une, il propose les jeux déjà lancés sur ce PC. Sinon, il passe directement à la recherche.
2. **Ajouter un jeu** : le joueur tape un nom, un AppID (`1245620`) ou colle un lien du magasin Steam. Le logiciel affiche les résultats avec leur jaquette.
3. **Import** : un clic importe tous les succès (nom, description, icônes, ID technique, rareté mondiale). Les succès déjà obtenus sont cochés automatiquement si l'info est disponible (cache local ou clé API), sinon le joueur coche à la main.
4. **Choisir ses objectifs** : depuis la liste complète, le joueur glisse ou sélectionne les succès sur lesquels il veut travailler. Ils rejoignent son espace de travail.
5. **Construire ses guides** : sous chaque succès, un bouton « + » ajoute un bloc : vidéo YouTube, image, texte, liste, compteur ou chronomètre. Les blocs se réorganisent par glisser-déposer.
6. **Jouer** : le joueur coche ses succès, fait +1 sur ses compteurs ou lance un chrono, éventuellement en mode compact ou par raccourci clavier sans quitter le jeu.
7. **Changer de jeu** : la barre latérale liste les jeux importés. Passer de l'un à l'autre ne perd rien.
8. **100 %** : le dernier succès coché déclenche la célébration, et le jeu rejoint la vitrine des trophées.

## Fonctionnalités V1

La V1 couvre l'import sans clé, l'espace de travail par succès, les six types de blocs, la sauvegarde et la célébration du 100 %.

### Bibliothèque de jeux

- Ajout par nom, AppID ou lien Steam (formats `store.steampowered.com/app/<id>`, `steamcommunity.com/stats/<id>`, `steam://`).
- Barre latérale des jeux importés : jaquette, nom, progression (x / y, %).
- Supprimer un jeu demande confirmation et propose d'exporter ses guides avant.
- Bouton « Actualiser » pour réimporter les succès (nouveaux succès, progression) sans toucher aux guides.

### Liste des succès

- Chaque succès affiche : icône (couleur si obtenu, grise sinon), nom, description, **ID technique** (copiable en un clic), rareté mondiale en %, date d'obtention si connue.
- Coche manuelle obtenu / non obtenu, toujours possible même avec synchronisation.
- **Mode anti-spoiler** (activé par défaut) : nom et description des succès cachés floutés jusqu'au clic.
- **Recherche et filtres** : texte, obtenu / non obtenu, étiquette, tranche de rareté. Tri par nom, rareté, ordre Steam ou ordre personnel.

### Espace de travail

- Le joueur glisse ou sélectionne (case à cocher, sélection multiple) les succès à travailler.
- Chaque succès devient une **carte** indépendante, repliable, déplaçable.
- Deux vues : **liste** (cartes empilées) et **colonnes** (à faire / en cours / fait, par glisser-déposer). On gardera la plus réussie si l'une déçoit.

### Blocs sous chaque succès (bouton « + »)

| Bloc | Ce qu'il fait |
| --- | --- |
| Vidéo YouTube | Colle un lien, lecteur intégré. **Repères de temps** : « 3:42 → le coffre caché », un clic saute au moment. |
| Image | Fichier, glisser-déposer ou **Ctrl+V** d'une capture. Légende facultative, clic pour agrandir. |
| Texte | Texte libre avec mise en forme simple (gras, titres, listes, liens). |
| Liste | Cases à cocher (collectibles, étapes), avec compteur x / y. |
| Compteur | Valeur actuelle / objectif (ex. 37 / 50), boutons +1 / −1, barre de progression. |
| Chronomètre | Chrono avec temps cible facultatif (ex. finir un niveau en moins de 5 min), meilleur temps conservé. |

Les blocs se réordonnent, se dupliquent et se suppriment (avec annulation possible).

### Étiquettes

- Le joueur pose les étiquettes de son choix sur un succès.
- Quelques étiquettes proposées au départ (ratable, grind, en ligne, difficile, cumulable, DLC), modifiables ou supprimables.
- Un menu Réglages permet d'en créer, renommer, supprimer et choisir leur couleur.

### Raccourcis clavier globaux

- Fonctionnent même quand le jeu est au premier plan : +1 / −1 sur le compteur « actif », démarrer / arrêter / remettre à zéro le chrono « actif ».
- Le joueur choisit le compteur ou chrono actif (icône épingle) et configure les touches dans Réglages.
- Un petit retour visuel ou sonore facultatif confirme l'action.

### Mode compact (option)

- Fenêtre réduite « toujours au premier plan » pour un 2ᵉ écran ou par-dessus un jeu en fenêtré : succès épinglés, compteurs et chronos.
- Opacité réglable.

### Re-lock

- Assistant discret qui affiche les commandes console Steam (détail plus bas).

### Sauvegarde

- Enregistrement automatique à chaque modification, sans bouton « Enregistrer ».
- Chaque jeu garde ses guides, notes et étiquettes indépendamment des autres.

### Partage de guides

- **Exporter** les guides d'un jeu dans un fichier `.json` (images incluses, ou dans une archive `.zip`).
- **Importer** un guide : il se fusionne avec le jeu existant ou crée le jeu s'il n'existe pas. Les blocs sont rattachés aux succès par leur ID technique.

### Langue et apparence

- Interface en français et en anglais (langue du système par défaut).
- Noms et descriptions des succès importés dans la langue choisie, si Steam la fournit.
- Thème sombre (par défaut) et clair.

## Fonctionnalités V2 et suite

Ces fonctions viennent une fois la V1 stable. L'architecture doit déjà les permettre.

- **Accueil multi-jeux** : jeux presque finis, progression globale, derniers succès obtenus, date de chaque 100 %.
- **Alerte nouveaux succès** : à l'actualisation, si un jeu à 100 % a reçu de nouveaux succès (DLC, mise à jour), il perd son statut doré et une alerte l'indique.
- **Couleurs reprises de la jaquette** : l'accent de l'interface s'adapte aux couleurs dominantes du jeu ouvert.
- **Sauvegardes de sécurité** : copie automatique des données (ex. une par jour, les 7 dernières gardées) et restauration depuis Réglages.
- **Mode portable** : tout le logiciel et ses données dans un seul dossier (clé USB, Steam Deck).
- **Autres langues** : traductions ajoutées par la communauté via des fichiers de langue.
- **Bibliothèque de guides partagés** (piste lointaine) : un dépôt communautaire où télécharger les guides exportés par d'autres joueurs.

## Sources de données Steam

L'import combine plusieurs sources publiques, sans clé, par ordre de priorité. La clé API ne fait qu'ajouter la synchronisation de tous les jeux. Les formats non officiels sont à valider dès le premier prototype.

| Source | Ce qu'elle apporte | Clé ? | Limites |
| --- | --- | --- | --- |
| Cache local de Steam : `appcache/stats/UserGameStatsSchema_<appid>.bin` | Liste complète : ID technique, noms et descriptions (toutes langues), icônes, succès cachés compris | Non | Seulement les jeux déjà lancés sur ce PC. Format binaire non documenté (KeyValues). |
| Cache local : `appcache/stats/UserGameStats_<accountid>_<appid>.bin` | Succès débloqués et date d'obtention | Non | Même limite. Plusieurs comptes possibles sur le PC : choisir le bon. |
| Cache local : `userdata/<accountid>/config/localconfig.vdf` | Temps de jeu par jeu (carte du 100 %) | Non | Format texte VDF, simple à lire. |
| Profil public : `steamcommunity.com/profiles/<steamid>/stats/<appid>/achievements/?xml=1` | ID technique, noms, descriptions, icônes, débloqué ou non, date | Non | Profil et détails de jeu publics. Le joueur colle juste son lien de profil. |
| Page publique : `steamcommunity.com/stats/<appid>/achievements/` | Noms, descriptions, icônes, rareté, pour n'importe quel jeu | Non | Pas d'ID technique. Descriptions des succès cachés absentes. Lecture de page HTML, donc fragile. |
| `api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/?gameid=<appid>` | Rareté mondiale par ID technique | Non | Pourcentages seulement. |
| `store.steampowered.com/api/storesearch/?term=<nom>` et `/api/appdetails?appids=<appid>` | Recherche par nom, nom officiel, images | Non | Limite de requêtes, à ménager (cache). |
| CDN Steam : `steam/apps/<appid>/library_600x900.jpg`, `header.jpg`, `library_hero.jpg` | Jaquettes de bibliothèque | Non | Certains vieux jeux n'ont pas la jaquette verticale : prévoir un repli sur `header.jpg`. |
| API Web : `GetSchemaForGame` + `GetPlayerAchievements` | Schéma et progression pour toute la bibliothèque | **Oui** (option) | La clé est stockée en local, jamais envoyée ailleurs qu'à Steam. |

**Ordre d'import pour un jeu**

1. Cache local si le jeu y figure (le plus complet).
2. Sinon, profil public si le joueur a renseigné son lien de profil.
3. Sinon, clé API si elle est configurée.
4. Sinon, page publique : l'import fonctionne, mais sans ID technique. Le logiciel le signale et retente dès qu'une meilleure source est disponible (le jeu lancé une fois suffit).

La rareté vient toujours de l'API publique des pourcentages. Les réponses réseau et les images sont mises en cache pour fonctionner hors ligne.

**Emplacements de Steam à détecter** : Windows via le registre (`HKCU\Software\Valve\Steam\SteamPath`), sinon `C:\Program Files (x86)\Steam`. Linux : `~/.steam/steam`, `~/.local/share/Steam`, et Flatpak `~/.var/app/com.valvesoftware.Steam/.local/share/Steam`. Chemin personnalisable dans Réglages.

## Re-lock : assistant de commandes Steam

Le logiciel ne touche jamais aux succès : il prépare la commande, l'utilisateur la colle lui-même dans la console Steam. L'option reste discrète (menu « … » d'un succès ou d'un jeu) pour ne pas encombrer l'interface.

**Re-lock d'un succès** (nécessite son ID technique)

```
achievement_clear <appid> <ID_technique>
```

**Reset complet d'un jeu** (tous les succès et statistiques)

```
reset_all_stats <appid>
```

**Ce que la fenêtre affiche**

- La commande déjà remplie, avec un bouton **Copier**.
- Un bouton **Ouvrir la console Steam** qui lance `steam://open/console`.
- La démarche, sous forme de cases à cocher :
    1. Fermer le jeu.
    2. Ouvrir la console Steam et coller la commande.
    3. Pour un reset complet : désactiver Steam Cloud pour ce jeu (Propriétés → Général), sinon les anciennes sauvegardes reviennent.
    4. Supprimer les sauvegardes locales du jeu (lien vers la page PCGamingWiki du jeu, qui indique leur emplacement).
    5. Relancer le jeu et vérifier que les succès sont bien reverrouillés.
- Un avertissement : action irréversible. Certains jeux redébloquent les succès depuis leur sauvegarde ou leurs serveurs.
- Si l'ID technique est inconnu (jeu importé depuis la page publique), le bouton est grisé avec l'explication.

Après confirmation, le logiciel propose de décocher les succès concernés, sans toucher aux guides.

## Interface et expérience

L'interface vise l'esprit d'une bibliothèque de jeux moderne : sombre, aérée, portée par les jaquettes, avec des animations courtes et fluides.

**Disposition de l'écran principal**

- **À gauche** : barre des jeux importés (jaquette verticale, nom, anneau de progression), bouton « Ajouter un jeu ».
- **En haut** : bannière du jeu ouvert (image `library_hero`), progression x / y, bouton Actualiser.
- **Au centre** : l'espace de travail (cartes des succès choisis, vue liste ou colonnes).
- **À droite, repliable** : la liste complète des succès du jeu, avec recherche et filtres, d'où l'on glisse vers le centre.

**Principes visuels**

- Succès obtenu : icône en couleur, légère lueur. Non obtenu : icône grise.
- Rareté en couleur (ex. moins de 5 % = or, moins de 20 % = violet, sinon neutre), toujours accompagnée du chiffre.
- Glisser-déposer avec aperçu de la carte et zone d'arrivée visible.
- Accessibilité : contraste suffisant, navigation au clavier, animations désactivables.

**Cocher un succès**

- Petite animation (l'icône s'allume, la coche se dessine), son discret facultatif.

**Le 100 %**

- Pluie de confettis aux couleurs de la jaquette du jeu.
- La carte du jeu passe en **version dorée** : bordure et reflet dorés, badge « 100 % ».
- Récap affiché : date du 100 %, temps de jeu Steam, succès le plus rare obtenu.
- Le jeu rejoint la **vitrine des trophées** : une galerie des jeux terminés, jaquettes dorées, triées par date.
- La célébration ne se déclenche qu'une fois par jeu (rejouable depuis la vitrine).

## Choix techniques et architecture

Tauri 2 est le meilleur choix : un installeur d'environ 10 Mo, une faible consommation de mémoire pendant qu'on joue, et le même code pour Windows et Linux.

| Brique | Choix | Pourquoi |
| --- | --- | --- |
| Cadre de l'application | Tauri 2 (cœur en Rust) | Léger, multiplateforme, accès natif aux fichiers et au clavier. Electron marcherait aussi mais pèse environ 10 fois plus. |
| Interface | Svelte 5 + TypeScript + Tailwind CSS | Rapide à écrire, animations fluides intégrées. |
| Glisser-déposer | `svelte-dnd-action` | Listes et colonnes, accessible au clavier. |
| Données | SQLite via `tauri-plugin-sql` | Un seul fichier, robuste, migrations faciles. |
| Lecture Steam | Module Rust dédié (`keyvalues-parser` pour les VDF texte, lecteur maison pour les `.bin`) | Isolé du reste, réparable seul si Valve change un format. |
| Raccourcis globaux | `tauri-plugin-global-shortcut` | Sous Linux Wayland, dépend du portail de raccourcis du bureau (KDE, GNOME récent) : à tester tôt. |
| Vidéo | Lecteur YouTube intégré (`youtube-nocookie.com`, API IFrame pour les repères) | Sous Linux, la lecture dépend des codecs GStreamer installés : afficher un lien de secours. |
| Confettis | `canvas-confetti` | Léger, couleurs personnalisables. |
| Traduction | `svelte-i18n`, un fichier par langue | Les contributeurs ajoutent une langue sans toucher au code. |
| Distribution | GitHub Actions + `tauri-action` | Installeurs Windows générés à chaque version. |

**Architecture**

```
┌─ Interface (Svelte + TypeScript) ─────────────────────────────────┐
│  Bibliothèque │ Succès et filtres │ Espace et blocs │ Réglages,   │
│               │                   │                 │ compact     │
└───────────────────────────────┬───────────────────────────────────┘
                                │ commandes Tauri
┌─ Cœur (Rust, Tauri 2) ────────▼───────────────────────────────────┐
│  Module Steam                      │ Base SQLite    │ Raccourcis   │
│  (cache local, profil public,      │ (guides,       │ globaux      │
│   page publique)                   │  export)       │ (+1, chrono) │
└──────┬────────────────────┬────────┴───────┬────────┴──────────────┘
       │ lecture seule      │ HTTPS, sans clé│ écriture auto
       ▼                    ▼                ▼
  Steam installé      Web Steam public   Dossier de données
  (appcache,          (magasin, %,       (SQLite + images)
   userdata)           jaquettes)
```

L'interface ne parle jamais directement à Steam : elle passe par le cœur Rust, qui lit Steam sans y écrire et enregistre tout dans le dossier de données.

## Modèle de données et sauvegarde

Toutes les données tiennent dans une base SQLite locale, plus un dossier d'images. Chaque guide est rattaché à un jeu (AppID) et à un succès (ID technique), ce qui isole les jeux entre eux et rend l'export simple.

| Table | Champs principaux | Rôle |
| --- | --- | --- |
| `games` | `appid`, nom, chemins des jaquettes, source d'import, date d'import, date du 100 %, temps de jeu, ordre dans la barre | Un jeu importé |
| `achievements` | `appid`, `api_name` (ID technique, peut être vide), nom, description, caché oui/non, icônes, rareté %, obtenu oui/non, date d'obtention, coché à la main oui/non | Les succès d'un jeu, mis à jour par Actualiser |
| `workspace_items` | `appid`, succès, colonne (à faire / en cours / fait), position, replié oui/non | Les succès choisis et leur place |
| `blocks` | `id`, succès, type (`video`, `image`, `text`, `list`, `counter`, `timer`), position, `data` (JSON) | Le contenu des guides |
| `tags` et `achievement_tags` | nom, couleur ; lien succès ↔ étiquette | Étiquettes du joueur |
| `settings` | clé, valeur | Langue, thème, raccourcis, chemin Steam, lien de profil, clé API |

**Contenu `data` par type de bloc**

- `video` : URL, liste de repères `{temps_en_secondes, libellé}`.
- `image` : nom du fichier dans `images/`, légende.
- `text` : texte en Markdown.
- `list` : éléments `{texte, coché}`.
- `counter` : valeur, objectif, pas.
- `timer` : temps cible, meilleur temps, temps en cours.

**Règles de sauvegarde**

- Écriture immédiate à chaque modification (avec un léger délai groupé pour la frappe au clavier).
- Dossier de données standard du système : `%APPDATA%\<nom-du-logiciel>` sous Windows, `~/.local/share/<nom-du-logiciel>` sous Linux.
- Une actualisation des succès met à jour `achievements` sans jamais toucher à `workspace_items`, `blocks` ni aux étiquettes. Un succès disparu de Steam garde ses guides, marqué « retiré ».
- Numéro de version du schéma et migrations automatiques, pour ne jamais casser les données d'une version à l'autre.

**Format d'export d'un guide** : un fichier JSON avec la version du format, l'AppID, et pour chaque succès son ID technique (ou à défaut son nom), ses étiquettes et ses blocs. Les images vont dans une archive `.zip` à côté du JSON. Jamais de données personnelles (progression, clé API, profil) dans un export.

## Étapes de développement avec Claude Code

On avance en neuf étapes, chacune livrant un logiciel qui fonctionne et se teste avant de passer à la suivante.

**Mise en route** : placer ce fichier à la racine du projet sous le nom `SPEC.md`, puis demander à Claude Code de créer un `CLAUDE.md` qui y renvoie. Donner ensuite une étape à la fois : « Réalise l'étape 2 du SPEC.md ».

1. **Squelette** : projet Tauri 2 + Svelte + TypeScript, dépôt Git, licence, README. *Validé quand* une fenêtre vide s'ouvre sous Windows 11.
2. **Prototype des sources Steam** : détection de Steam, lecture du cache local (schéma + progression), du profil public XML et de la page publique, sur 3 jeux tests. *Validé quand* les succès (avec ID technique) et leur état s'affichent en texte brut. C'est l'étape la plus risquée : on la fait tôt.
3. **Base de données et bibliothèque** : SQLite, ajout d'un jeu par nom / AppID / lien, barre latérale, jaquettes. *Validé quand* les jeux importés réapparaissent après redémarrage.
4. **Liste des succès** : affichage complet, coche manuelle, rareté, anti-spoiler, recherche et filtres.
5. **Espace de travail** : sélection et glisser-déposer des succès, cartes, vue liste puis vue colonnes.
6. **Blocs** : texte, liste, compteur, image (avec Ctrl+V), vidéo YouTube avec repères, chronomètre. *Validé quand* tout est retrouvé à l'identique après redémarrage et en changeant de jeu.
7. **Étiquettes, re-lock, raccourcis globaux, mode compact**.
8. **Finitions** : 100 % et vitrine, animations, thèmes, traduction français / anglais, export / import de guides, clé API optionnelle.
9. **Publication** : compilation automatique par GitHub Actions pour Windows (`.msi` / `.exe`), page de release sur GitHub. Pas de version Linux (voir Vision).

**Règles à rappeler à Claude Code**

- Ne jamais écrire dans les fichiers de Steam : lecture seule.
- Ne jamais exécuter de commande console Steam : seulement les afficher.
- Tester chaque étape sur les deux systèmes avant de continuer.
- Garder la logique Steam isolée dans un module, pour pouvoir la réparer seule si Valve change un format.

**Questions ouvertes**

- Nom du logiciel.
- Faut-il gérer plusieurs comptes Steam sur un même PC dès la V1 ?
- Le lecteur YouTube intégré doit-il continuer à jouer en mode compact ?
