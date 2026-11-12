// Étiquettes : définitions communes à tous les jeux, et celles posées sur les succès du jeu ouvert.
import { invoke } from "@tauri-apps/api/core";

export interface Tag {
  id: number;
  name: string;
  /** #rrggbb */
  color: string;
}

/** Couleurs proposées dans les Réglages et à la création rapide. */
export const TAG_COLORS = ["#ef5b5b", "#f2994a", "#f2c14e", "#4cc38a", "#2bb5c8", "#4f7cff", "#b07cff", "#e86bb0", "#8b93a7"];

export const tags = $state({
  list: [] as Tag[],
  /** Étiquettes posées, par succès (jeu ouvert). */
  byAch: {} as Record<number, number[]>,
  appid: 0,
  error: "",
});

function fail(e: unknown) {
  tags.error = String(e);
}

export async function loadTags() {
  try {
    tags.list = await invoke<Tag[]>("tags_list");
  } catch (e) {
    fail(e);
  }
}

export async function loadGameTags(appid: number) {
  tags.appid = appid;
  tags.byAch = {};
  try {
    const rows = await invoke<{ achievement_id: number; tag_id: number }[]>("achievement_tags_list", { appid });
    if (tags.appid !== appid) return;
    const map: Record<number, number[]> = {};
    for (const r of rows) (map[r.achievement_id] ??= []).push(r.tag_id);
    tags.byAch = map;
  } catch (e) {
    fail(e);
  }
}

/** Étiquettes d'un succès, dans l'ordre des Réglages. */
export function tagsOf(achievementId: number): Tag[] {
  const ids = tags.byAch[achievementId];
  if (!ids?.length) return [];
  return tags.list.filter((t) => ids.includes(t.id));
}

export const hasTag = (achievementId: number, tagId: number) => tags.byAch[achievementId]?.includes(tagId) ?? false;

/** Pose ou retire une étiquette (affichage immédiat, annulé si l'enregistrement échoue). */
export async function toggleTag(achievementId: number, tagId: number) {
  const on = !hasTag(achievementId, tagId);
  const before = tags.byAch[achievementId] ?? [];
  tags.byAch[achievementId] = on ? [...before, tagId] : before.filter((x) => x !== tagId);
  try {
    await invoke("achievement_tag_set", { achievementId, tagId, on });
  } catch (e) {
    tags.byAch[achievementId] = before;
    fail(e);
  }
}

/** Crée une étiquette. Renvoie un message d'erreur pour le joueur, ou null. */
export async function createTag(name: string, color: string): Promise<Tag | string> {
  try {
    const t = await invoke<Tag>("tag_create", { name, color });
    tags.list.push(t);
    return t;
  } catch (e) {
    return String(e);
  }
}

export async function updateTag(id: number, name: string, color: string): Promise<string | null> {
  try {
    await invoke("tag_update", { id, name, color });
    const t = tags.list.find((x) => x.id === id);
    if (t) Object.assign(t, { name: name.trim(), color });
    return null;
  } catch (e) {
    return String(e);
  }
}

export async function deleteTag(id: number): Promise<string | null> {
  try {
    await invoke("tag_delete", { id });
    tags.list = tags.list.filter((t) => t.id !== id);
    for (const k of Object.keys(tags.byAch)) {
      const n = Number(k);
      tags.byAch[n] = tags.byAch[n].filter((x) => x !== id);
    }
    return null;
  } catch (e) {
    return String(e);
  }
}

/** Couleur de texte lisible sur le fond de l'étiquette. */
export function tagStyle(color: string): string {
  return `background: color-mix(in srgb, ${color} 22%, transparent); color: color-mix(in srgb, ${color} 75%, white); border-color: color-mix(in srgb, ${color} 45%, transparent)`;
}
