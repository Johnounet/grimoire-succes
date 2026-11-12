// Commandes de la bibliothèque (miroir de src-tauri/src/library.rs et db.rs).
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
}

export const api = {
  imagesDir: () => invoke<string>("images_dir"),
  list: () => invoke<GameSummary[]>("library_list"),
  localSuggestions: () => invoke<SearchHit[]>("library_local_suggestions"),
  search: (query: string) => invoke<SearchHit[]>("library_search", { query }),
  importGame: (appid: number) => invoke<GameSummary>("library_import", { appid }),
  deleteGame: (appid: number) => invoke<void>("library_delete", { appid }),
  achievements: (appid: number) => invoke<AchievementRow[]>("game_achievements", { appid }),
  setUnlocked: (id: number, unlocked: boolean) =>
    invoke<GameSummary>("achievement_set_unlocked", { id, unlocked }),
  workspaceList: (appid: number) => invoke<WorkspaceItem[]>("workspace_list", { appid }),
  workspaceSave: (appid: number, items: WorkspaceItem[]) => invoke<void>("workspace_save", { appid, items }),
  getSetting: (key: SettingKey) => invoke<string | null>("setting_get", { key }),
  setSetting: (key: SettingKey, value: string) => invoke<void>("setting_set", { key, value }),
};

export type SettingKey = "anti_spoiler" | "language" | "country" | "profile_url" | "workspace_view" | "panel_open";

export type Column = "todo" | "doing" | "done";

/** Place d'une carte dans la vue Tableau : grille de 12 colonnes, hauteur en rangées. */
export interface GridPos {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface WorkspaceItem {
  achievement_id: number;
  /** Statut de la carte. */
  column: Column;
  collapsed: boolean;
  /** null : pas encore placée dans le tableau (une place libre lui sera trouvée). */
  grid: GridPos | null;
}

/** Statuts possibles d'une carte (anciennement les colonnes de la vue Colonnes). */
export const columns: { id: Column; label: string }[] = [
  { id: "todo", label: "À faire" },
  { id: "doing", label: "En cours" },
  { id: "done", label: "Fait" },
];

/** Type MIME du glisser natif depuis le panneau des succès vers l'espace de travail. */
export const DRAG_MIME = "application/x-chasse-succes-ids";

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

export const sourceLabel: Record<GameSummary["source"], string> = {
  local_cache: "cache local de Steam",
  public_profile: "profil Steam public",
  public_page: "page publique Steam",
};
