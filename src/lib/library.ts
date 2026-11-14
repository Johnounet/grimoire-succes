// Commandes de la bibliothèque (miroir de src-tauri/src/library.rs et db.rs).

/** Nom du logiciel et sous-titre (une seule source pour toute l'interface). */
export const APP_NAME = "Grimoire";
export const APP_TAGLINE = "Concoctez vos guides jusqu’au 100 %";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export interface GameSummary {
  appid: number;
  name: string;
  cover_file: string | null;
  cover_kind: "portrait" | "header" | null;
  source: "local_cache" | "public_profile" | "public_page";
  total: number;
  unlocked: number;
  has_api_names: boolean;
  refreshed_at: number;
  /** Date du 100 % (secondes Unix) ; null tant que le jeu n'est pas terminé. */
  completed_at: number | null;
  playtime_minutes: number | null;
  /** La célébration du 100 % a déjà été montrée (elle ne revient que depuis le badge « 100 % » du jeu). */
  celebrated: boolean;
}

/** Récapitulatif du 100 %. */
export interface Recap {
  game: GameSummary;
  rarest: AchievementRow | null;
}

export interface SearchHit {
  appid: number;
  name: string;
  capsule_url: string;
  in_library: boolean;
  on_this_pc: boolean;
}

export interface AchievementRow {
  id: number;
  api_name: string | null;
  name: string;
  description: string;
  hidden: boolean;
  icon_url: string | null;
  icon_gray_url: string | null;
  rarity: number | null;
  unlocked: boolean;
  unlock_time: number | null;
  manual: boolean;
  dlc_appid: number | null;
  removed: boolean;
  /** Progression suivie par Steam (cache local), dernière valeur connue. */
  progress: Progress | null;
}

/** Succès obtenu et daté, avec son jeu (page d'accueil). */
export interface RecentUnlock {
  id: number;
  appid: number;
  game_name: string;
  name: string;
  icon_url: string | null;
  rarity: number | null;
  unlock_time: number;
}

/** Sauvegarde de sécurité des données (backup.rs). */
export interface BackupInfo {
  id: string;
  created_at: number;
  kind: "auto" | "manuel" | "avant-restauration";
  games: number;
  /** Octets (base et images copiées). */
  size: number;
}

/** Barre de progression d'un succès : valeur d'une statistique du jeu entre `min` et `max`. */
export interface Progress {
  value: number;
  min: number;
  max: number;
}

export const api = {
  imagesDir: () => invoke<string>("images_dir"),
  list: () => invoke<GameSummary[]>("library_list"),
  /** Ordre des jeux du panneau de gauche, le premier en haut. */
  reorder: (appids: number[]) => invoke<void>("library_reorder", { appids }),
  localSuggestions: () => invoke<SearchHit[]>("library_local_suggestions"),
  search: (query: string) => invoke<SearchHit[]>("library_search", { query }),
  importGame: (appid: number) => invoke<GameSummary>("library_import", { appid }),
  deleteGame: (appid: number) => invoke<void>("library_delete", { appid }),
  achievements: (appid: number) => invoke<AchievementRow[]>("game_achievements", { appid }),
  setUnlocked: (id: number, unlocked: boolean) =>
    invoke<GameSummary>("achievement_set_unlocked", { id, unlocked }),
  lockAll: (appid: number) => invoke<GameSummary>("achievements_lock_all", { appid }),
  recap: (appid: number) => invoke<Recap>("game_recap", { appid }),
  markCelebrated: (appid: number) => invoke<void>("game_mark_celebrated", { appid }),
  /** Derniers succès débloqués, tous jeux confondus (page d'accueil). */
  recentUnlocks: (limit: number) => invoke<RecentUnlock[]>("home_recent_unlocks", { limit }),
  workspaceList: (appid: number) => invoke<WorkspaceItem[]>("workspace_list", { appid }),
  workspaceSave: (appid: number, items: WorkspaceItem[]) => invoke<void>("workspace_save", { appid, items }),
  workspaceCardSizes: (appid: number) =>
    invoke<{ achievement_id: number; w: number; h: number }[]>("workspace_card_sizes", { appid }),
  /** Exporte le guide (fenêtre « Enregistrer sous » de Windows). Chemin du fichier, ou null si annulé. */
  guideExport: (appid: number, keepState: boolean) => invoke<string | null>("guide_export", { appid, keepState }),
  /** Importe un guide (fenêtre « Ouvrir » de Windows). Bilan, ou null si annulé. */
  guideImport: () => invoke<ImportReport | null>("guide_import"),
  sections: (appid: number) => invoke<Section[]>("sections_list", { appid }),
  sectionCreate: (appid: number, title: string, color: SectionColor) =>
    invoke<Section>("section_create", { appid, title, color }),
  /** Titre, état replié, couleur et ordre (celui de la liste) des sections du jeu. */
  sectionsSave: (appid: number, sections: Section[]) => invoke<void>("sections_save", { appid, sections }),
  /** Supprime la section ; ses cartes encore dans l'espace passent hors section. */
  sectionDelete: (id: number) => invoke<void>("section_delete", { id }),
  freeCards: (appid: number) => invoke<FreeCard[]>("free_cards_list", { appid }),
  freeCardCreate: (appid: number, title: string) => invoke<FreeCard>("free_card_create", { appid, title }),
  freeCardRename: (id: number, title: string) => invoke<void>("free_card_rename", { id, title }),
  freeCardDelete: (id: number) => invoke<void>("free_card_delete", { id }),
  backups: () => invoke<BackupInfo[]>("backups_list"),
  backupCreate: () => invoke<BackupInfo>("backup_create"),
  backupRestore: (id: string) => invoke<void>("backup_restore", { id }),
  backupDelete: (id: string) => invoke<void>("backup_delete", { id }),
  backupsOpenFolder: () => invoke<void>("backups_open_folder"),
  /** Version portable (`portable.txt` à côté de l'exécutable) ou installée. */
  appMode: () => invoke<{ portable: boolean }>("app_mode"),
  /** Nouvelle version publiée sur GitHub, ou null si Grimoire est à jour. */
  updateCheck: () => invoke<UpdateInfo | null>("update_check"),
  /** Télécharge et installe la version trouvée ; l'application se ferme puis redémarre. */
  updateInstall: () => invoke<void>("update_install"),
  /** Ouvre la page de téléchargement des versions. */
  updateOpenPage: () => invoke<void>("update_open_page"),
  getSetting: (key: SettingKey) => invoke<string | null>("setting_get", { key }),
  setSetting: (key: SettingKey, value: string) => invoke<void>("setting_set", { key, value }),
};

/** Nouvelle version disponible (`update_check`). */
export interface UpdateInfo {
  version: string;
  notes: string | null;
  /** Installation en un clic (version installée) ; sinon, lien vers la page de téléchargement. */
  canInstall: boolean;
}

export type SettingKey =
  | "anti_spoiler"
  | "language"
  | "country"
  | "profile_url"
  | "workspace_view"
  | "panel_open"
  | "sidebar_open"
  | "shortcuts"
  | "shortcut_sound"
  | "compact_opacity"
  | "animations"
  | "check_sound"
  | "appearance"
  | "theme_dark"
  | "theme_light"
  | "last_game"
  | "update_check";

export type Column = "todo" | "doing" | "done";

/** Place d'une carte dans la vue Tableau : grille de 12 colonnes, hauteur en rangées. */
export interface GridPos {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Bilan de l'import d'un guide (guides.rs). */
export interface ImportReport {
  game: GameSummary;
  blocks: number;
  free_cards: number;
  placed: number;
  duplicates: number;
  /** Succès du guide introuvables dans ce jeu : leurs blocs sont dans une carte libre. */
  unmatched: string[];
  /** Étiquettes du guide refusées (nom vide ou trop long) : ignorées, le reste est importé. */
  ignored_tags: string[];
}

/** Phrase de bilan affichée après un import. */
export function importSummary(r: ImportReport): string {
  const parts: string[] = [];
  if (r.blocks) parts.push(`${r.blocks} bloc${r.blocks > 1 ? "s" : ""}`);
  if (r.free_cards) parts.push(`${r.free_cards} carte${r.free_cards > 1 ? "s" : ""} libre${r.free_cards > 1 ? "s" : ""}`);
  if (r.placed) parts.push(`${r.placed} carte${r.placed > 1 ? "s" : ""} dans l'espace`);
  let text = parts.length ? `Guide importé dans ${r.game.name} : ${parts.join(", ")}.` : `Rien de nouveau dans ce guide pour ${r.game.name}.`;
  if (r.duplicates) text += ` ${r.duplicates} élément${r.duplicates > 1 ? "s" : ""} déjà présent${r.duplicates > 1 ? "s" : ""}, ignoré${r.duplicates > 1 ? "s" : ""}.`;
  if (r.unmatched.length)
    text += ` ${r.unmatched.length} succès introuvable${r.unmatched.length > 1 ? "s" : ""} dans ce jeu : ${r.unmatched.length > 1 ? "leurs blocs sont rangés dans des cartes libres" : "ses blocs sont rangés dans une carte libre"}.`;
  if (r.ignored_tags.length) {
    const names = r.ignored_tags.map((n) => (n ? `« ${n.length > 20 ? n.slice(0, 20) + "…" : n} »` : "sans nom")).join(", ");
    text += ` ${r.ignored_tags.length > 1 ? "Étiquettes invalides ignorées" : "Étiquette invalide ignorée"} : ${names}.`;
  }
  return text;
}

/** Carte libre : un titre et des blocs, rattachée au jeu et non à un succès. */
export interface FreeCard {
  id: number;
  title: string;
}

/** Couleurs possibles d'une section (même liste que `SECTION_COLORS` côté Rust). La teinte dépend du thème :
 *  variable CSS `--section-<nom>` posée par theme.svelte.ts. */
export const SECTION_COLORS = ["violet", "bleu", "turquoise", "vert", "ambre", "corail", "rose", "gris"] as const;
export type SectionColor = (typeof SECTION_COLORS)[number];
export const sectionColorLabel: Record<SectionColor, string> = {
  violet: "Violet",
  bleu: "Bleu",
  turquoise: "Turquoise",
  vert: "Vert",
  ambre: "Ambre",
  corail: "Corail",
  rose: "Rose",
  gris: "Gris",
};
/** Valeur CSS de la couleur d'une section. */
export const sectionColor = (c: string) => `var(--section-${SECTION_COLORS.includes(c as SectionColor) ? c : "violet"})`;

/** Section de l'espace de travail : groupe de cartes nommé, repliable, coloré, déplaçable. */
export interface Section {
  id: number;
  title: string;
  collapsed: boolean;
  color: SectionColor;
}

/** Ce qui porte une carte de l'espace de travail ou un bloc : un succès ou une carte libre (l'un des deux). */
export interface CardOwner {
  achievement_id: number | null;
  card_id: number | null;
}

/**
 * Clé d'une carte dans l'interface, un seul nombre pour les deux sortes de cartes :
 * l'id du succès (positif), ou l'opposé de l'id de la carte libre (négatif).
 * Sert d'identifiant pour les listes, le tableau, les blocs groupés, etc.
 */
export type CardKey = number;

export const cardKey = (o: CardOwner): CardKey => (o.card_id != null ? -o.card_id : (o.achievement_id ?? 0));
export const isFreeKey = (key: CardKey) => key < 0;
/** Clé → forme attendue par le cœur Rust. */
export const ownerOf = (key: CardKey): CardOwner =>
  key < 0 ? { achievement_id: null, card_id: -key } : { achievement_id: key, card_id: null };

export interface WorkspaceItem extends CardOwner {
  /** Statut de la carte. */
  column: Column;
  collapsed: boolean;
  /** null : pas encore placée dans le tableau (une place libre lui sera trouvée). */
  grid: GridPos | null;
  /** Affichée dans le mode compact. */
  pinned: boolean;
  /** Section de la carte ; absente ou null : zone du haut, hors section. `grid` est relatif à la section. */
  section?: number | null;
  /** Interface seulement : taille voulue quand `grid` est null (taille mémorisée d'une carte remise). */
  size?: { w: number; h: number };
  /** Interface seulement : carte qui vient d'être ajoutée (le tableau l'agrandit à son contenu). */
  fresh?: boolean;
}

/** Dernière taille connue des cartes de succès du jeu ouvert dans le tableau, y compris celles retirées
 *  de l'espace de travail (table `card_sizes`), par id du succès. Tenu à jour par la vue du jeu. */
export const cardSizes = new Map<number, { w: number; h: number }>();

/** Statuts possibles d'une carte (anciennement les colonnes de la vue Colonnes). */
export const columns: { id: Column; label: string }[] = [
  { id: "todo", label: "À faire" },
  { id: "doing", label: "En cours" },
  { id: "done", label: "Fait" },
];

/** Type MIME du glisser natif depuis le panneau des succès vers l'espace de travail. */
/** Événement DOM (qui remonte) émis par un bloc dont le contenu vient de grandir : dans le Tableau,
 *  la carte qui le contient s'agrandit pour tout montrer si elle est trop petite. */
export const FIT_CARD_EVENT = "fit-card";

/** Événement DOM (qui remonte, `detail: { open }`) émis à l'ouverture et à la fermeture du menu
 *  « Ajouter un bloc » : le Tableau rend à la carte sa taille d'avant si aucun bloc n'a été ajouté. */
export const CARD_MENU_EVENT = "card-menu";

export const DRAG_MIME = "application/x-grimoire-ids";

/** URL affichable d'une image enregistrée dans le dossier de données. */
export function localImage(imagesDir: string, rel: string | null): string | null {
  if (!rel) return null;
  // Interface simulée : l'image est déjà une URL du navigateur
  if (rel.startsWith("blob:") || rel.startsWith("data:")) return rel;
  return convertFileSrc(`${imagesDir}/${rel}`);
}

export const heroUrl = (appid: number) =>
  `https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${appid}/library_hero.jpg`;

export const percent = (g: { total: number; unlocked: number }) =>
  g.total ? Math.floor((g.unlocked / g.total) * 100) : 0;

/** Jeu terminé à 100 % (date posée par le cœur Rust). */
export const isComplete = (g: GameSummary) => g.completed_at != null;

/** Place d'un jeu qui vient d'atteindre 100 % : juste après le dernier jeu en cours (comme
 *  `move_to_completed` côté Rust), `games` étant la liste sans ce jeu. */
export function completedSlot(games: GameSummary[]) {
  for (let i = games.length - 1; i >= 0; i--) if (!isComplete(games[i])) return i + 1;
  return 0;
}

export const formatDate = (unix: number) =>
  new Date(unix * 1000).toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });

/** Date proche dite en mots : « aujourd'hui », « hier », « il y a 3 jours », sinon la date. */
export function formatAgo(unix: number): string {
  const day = (t: number) => {
    const d = new Date(t);
    return Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) / 86_400_000;
  };
  const days = day(Date.now()) - day(unix * 1000);
  if (days <= 0) return "aujourd'hui";
  if (days === 1) return "hier";
  if (days < 7) return `il y a ${days} jours`;
  return `le ${formatDate(unix)}`;
}

export function formatPlaytime(minutes: number): string {
  if (minutes < 60) return `${minutes} min`;
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return m && h < 10 ? `${h} h ${String(m).padStart(2, "0")}` : `${h.toLocaleString("fr-FR")} h`;
}

export const sourceLabel: Record<GameSummary["source"], string> = {
  local_cache: "cache local de Steam",
  public_profile: "profil Steam public",
  public_page: "page publique Steam",
};
