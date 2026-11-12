// Types miroirs du module Rust `steam` (src-tauri/src/steam).
import { invoke } from "@tauri-apps/api/core";

export type Outcome<T> = { ok: T } | { err: string };

export interface Achievement {
  api_name: string | null;
  name: string;
  description: string;
  hidden: boolean;
  icon_url: string | null;
  icon_gray_url: string | null;
  rarity: number | null;
  unlocked: boolean | null;
  unlock_time: number | null;
  dlc_appid: number | null;
}

export interface GameAchievements {
  appid: number;
  game_name: string | null;
  source: "local_cache" | "public_profile" | "public_page";
  language: string;
  achievements: Achievement[];
}

export interface DetectReport {
  steam_path: string | null;
  accounts: { account_id: number; steamid64: number; persona_name: string | null; most_recent: boolean }[];
  cached_games: { appid: number; name: string | null }[];
}

export interface ProbeReport {
  appid: number;
  local_cache: Outcome<GameAchievements>;
  public_profile: Outcome<GameAchievements>;
  public_page: Outcome<GameAchievements>;
  rarity_api_count: Outcome<number>;
}

export const steamDetect = () => invoke<DetectReport>("steam_detect");

export const steamProbe = (appid: number, lang: string, profile: string) =>
  invoke<ProbeReport>("steam_probe", { appid, lang, profile });
