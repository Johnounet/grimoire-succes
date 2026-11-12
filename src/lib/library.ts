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
  getSetting: (key: SettingKey) => invoke<string | null>("setting_get", { key }),
  setSetting: (key: SettingKey, value: string) => invoke<void>("setting_set", { key, value }),
};

export type SettingKey = "anti_spoiler" | "language" | "country" | "profile_url";

/** URL affichable d'une image enregistrée dans le dossier de données. */
export function localImage(imagesDir: string, rel: string | null): string | null {
  if (!rel) return null;
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
