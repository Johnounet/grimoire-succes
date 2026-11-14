// Recherche d'aide pour un succès : adresses YouTube, guides Steam et Google, ouvertes dans le navigateur
// par défaut. Ces trois adresses doivent rester autorisées dans `capabilities/default.json` (opener).
import { openUrl } from "@tauri-apps/plugin-opener";
import { getContext, setContext } from "svelte";

/** Jeu affiché, transmis aux boutons d'aide des cartes et de la liste sans passer par chaque composant. */
export interface HelpGame {
  appid: number;
  name: string;
}

const KEY = Symbol("help-game");

/** À appeler dans la vue du jeu : `get` renvoie le jeu courant (lu au clic, donc toujours à jour). */
export function provideHelpGame(get: () => HelpGame) {
  setContext(KEY, get);
}

export function helpGame(): (() => HelpGame) | undefined {
  return getContext(KEY);
}

/** Une destination du menu « Aide ». L'adresse n'est construite qu'au clic (`url`), pas à chaque affichage. */
export interface HelpTarget {
  label: string;
  hint: string;
  url: (game: HelpGame, achievement: string) => string;
}

const q = encodeURIComponent;
const clean = (s: string) => s.replace(/"/g, "").trim();

// « succès » : les noms sont le plus souvent importés en français, et le mot écarte les homonymes
export const HELP_TARGETS: HelpTarget[] = [
  {
    label: "Vidéos YouTube",
    hint: "Recherche du jeu et du succès",
    // Sans guillemets : avec, YouTube ne renvoie presque rien (essai de l'utilisateur, 2026-11-14)
    url: (g, a) => `https://www.youtube.com/results?search_query=${q(`${g.name} ${clean(a)} succès`)}`,
  },
  {
    label: "Guides Steam",
    hint: "Guides de la communauté qui parlent de ce succès",
    url: (g, a) => `https://steamcommunity.com/app/${g.appid}/guides/?searchText=${q(a.trim())}&browsefilter=trend`,
  },
  {
    label: "Recherche Google",
    hint: "Recherche du jeu et du succès",
    // Nom entre guillemets : Google garde le nom entier au lieu de mots épars
    url: (g, a) => `https://www.google.com/search?q=${q(`${g.name} "${clean(a)}" succès`)}`,
  },
];

export function openHelp(url: string) {
  // Hors de Tauri (interface simulée), l'ouverture passe par le navigateur
  openUrl(url).catch(() => window.open(url, "_blank", "noopener"));
}
