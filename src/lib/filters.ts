// Recherche, filtres et tris de la liste des succès (logique pure, sans interface).
import type { AchievementRow } from "./library";

export type StateFilter = "all" | "unlocked" | "locked";
export type RarityFilter = "all" | "ultra" | "rare" | "uncommon" | "common";
export type SortKey = "steam" | "name" | "rarest" | "commonest" | "recent";

export interface Filters {
  text: string;
  state: StateFilter;
  rarity: RarityFilter;
  sort: SortKey;
}

export const defaultFilters = (): Filters => ({ text: "", state: "all", rarity: "all", sort: "steam" });

export const rarityTiers: { value: RarityFilter; label: string }[] = [
  { value: "all", label: "Toutes les raretés" },
  { value: "ultra", label: "Ultra rares (< 5 %)" },
  { value: "rare", label: "Rares (5 – 20 %)" },
  { value: "uncommon", label: "Peu communs (20 – 50 %)" },
  { value: "common", label: "Communs (≥ 50 %)" },
];

export const sortOptions: { value: SortKey; label: string }[] = [
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

/** Sans accents ni majuscules, pour une recherche tolérante. */
const fold = (s: string) =>
  s
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();

export function matches(a: AchievementRow, f: Filters): boolean {
  if (f.state === "unlocked" && !a.unlocked) return false;
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

/** Filtre et trie. L'ordre Steam d'origine est celui du tableau reçu. Les succès retirés restent en fin de liste. */
export function apply(list: AchievementRow[], f: Filters): AchievementRow[] {
  const out = list.filter((a) => matches(a, f));
  const rarity = (a: AchievementRow, missing: number) => a.rarity ?? missing;
  const cmp: Record<SortKey, ((x: AchievementRow, y: AchievementRow) => number) | null> = {
    steam: null,
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

export const isDefault = (f: Filters) => f.text.trim() === "" && f.state === "all" && f.rarity === "all";
