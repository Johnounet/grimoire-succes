// Recherche, filtres et tris de la liste des succès (logique pure, sans interface).
import type { AchievementRow } from "./library";

export type StateFilter = "all" | "unlocked" | "locked";
export type RarityFilter = "all" | "ultra" | "rare" | "uncommon" | "common";
export type SortKey = "steam" | "personal" | "name" | "rarest" | "commonest" | "recent";

/** Étiquette : « all », « none » (sans étiquette) ou l'id d'une étiquette. */
export type TagFilter = "all" | "none" | number;

export interface Filters {
  text: string;
  state: StateFilter;
  rarity: RarityFilter;
  tag: TagFilter;
  sort: SortKey;
}

export const defaultFilters = (): Filters => ({ text: "", state: "all", rarity: "all", tag: "all", sort: "steam" });

/** Étiquettes posées, par succès. */
export type TagMap = Record<number, number[]>;

export const rarityTiers: { value: RarityFilter; label: string }[] = [
  { value: "all", label: "Toutes les raretés" },
  { value: "ultra", label: "Ultra rares (< 5 %)" },
  { value: "rare", label: "Rares (5 – 20 %)" },
  { value: "uncommon", label: "Peu communs (20 – 50 %)" },
  { value: "common", label: "Communs (≥ 50 %)" },
];

export const sortOptions: { value: SortKey; label: string }[] = [
  { value: "personal", label: "Mon ordre" },
  { value: "steam", label: "Ordre Steam" },
  { value: "name", label: "Nom" },
  { value: "rarest", label: "Plus rares d'abord" },
  { value: "commonest", label: "Plus communs d'abord" },
  { value: "recent", label: "Obtenus récemment" },
];

export function rarityTier(r: number | null): Exclude<RarityFilter, "all"> | null {
  if (r == null) return null;
  if (r < 5) return "ultra";
  if (r < 20) return "rare";
  if (r < 50) return "uncommon";
  return "common";
}

/** Couleur du texte de rareté : or sous 5 %, violet sous 20 %, neutre sinon. */
export function rarityText(r: number | null): string {
  const t = rarityTier(r);
  return t === "ultra" ? "text-gold" : t === "rare" ? "text-rare" : "text-muted";
}

export function rarityBar(r: number | null): string {
  const t = rarityTier(r);
  return t === "ultra" ? "bg-gold" : t === "rare" ? "bg-rare" : "bg-muted/60";
}

/** Sans accents ni majuscules, pour une recherche tolérante. */
const fold = (s: string) =>
  s
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();

export function matches(a: AchievementRow, f: Filters, tagMap: TagMap = {}): boolean {
  if (f.state === "unlocked" && !a.unlocked) return false;
  if (f.tag !== "all") {
    const mine = tagMap[a.id] ?? [];
    if (f.tag === "none" ? mine.length > 0 : !mine.includes(f.tag)) return false;
  }
  if (f.state === "locked" && a.unlocked) return false;
  if (f.rarity !== "all" && rarityTier(a.rarity) !== f.rarity) return false;
  const q = fold(f.text.trim());
  if (q) {
    const hay = fold(`${a.name} ${a.description} ${a.api_name ?? ""}`);
    if (!q.split(/\s+/).every((w) => hay.includes(w))) return false;
  }
  return true;
}

const byName = new Intl.Collator("fr", { sensitivity: "base", numeric: true });

/**
 * Filtre et trie. L'ordre Steam d'origine est celui du tableau reçu. Les succès retirés restent en fin de liste.
 * `personal` : rang de chaque succès dans l'espace de travail (ordre personnel) ; les autres suivent dans l'ordre Steam.
 */
export function apply(
  list: AchievementRow[],
  f: Filters,
  personal?: Map<number, number>,
  tagMap: TagMap = {},
): AchievementRow[] {
  const out = list.filter((a) => matches(a, f, tagMap));
  const rarity = (a: AchievementRow, missing: number) => a.rarity ?? missing;
  const rank = (a: AchievementRow) => personal?.get(a.id) ?? Infinity;
  const cmp: Record<SortKey, ((x: AchievementRow, y: AchievementRow) => number) | null> = {
    steam: null,
    personal: (x, y) => (rank(x) === rank(y) ? 0 : rank(x) - rank(y)),
    name: (x, y) => byName.compare(x.name, y.name),
    rarest: (x, y) => rarity(x, Infinity) - rarity(y, Infinity),
    commonest: (x, y) => rarity(y, -Infinity) - rarity(x, -Infinity),
    recent: (x, y) => (y.unlock_time ?? (y.unlocked ? 0 : -1)) - (x.unlock_time ?? (x.unlocked ? 0 : -1)),
  };
  const c = cmp[f.sort];
  // Array.prototype.sort est stable : à égalité, l'ordre Steam est conservé
  return out.sort((x, y) => Number(x.removed) - Number(y.removed) || (c ? c(x, y) : 0));
}

export function counts(list: AchievementRow[]) {
  const live = list.filter((a) => !a.removed);
  const unlocked = live.filter((a) => a.unlocked).length;
  return { all: live.length, unlocked, locked: live.length - unlocked };
}

export const isDefault = (f: Filters) =>
  f.text.trim() === "" && f.state === "all" && f.rarity === "all" && f.tag === "all";
