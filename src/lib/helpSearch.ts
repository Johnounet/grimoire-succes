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

export interface HelpLink {
  label: string;
  hint: string;
  url: string;
}

/** Nom du succès entre guillemets : le moteur garde le nom entier au lieu de mots épars. */
const quoted = (s: string) => `"${s.replace(/"/g, "").trim()}"`;

export function helpLinks(game: HelpGame, achievement: string): HelpLink[] {
  const q = encodeURIComponent;
  // « succès » : les noms sont le plus souvent importés en français, et le mot écarte les homonymes
  const query = `${game.name} ${quoted(achievement)} succès`;
  return [
    {
      label: "Vidéos YouTube",
      hint: "Recherche du jeu et du succès",
      url: `https://www.youtube.com/results?search_query=${q(query)}`,
    },
    {
      label: "Guides Steam",
      hint: "Guides de la communauté qui parlent de ce succès",
      url: `https://steamcommunity.com/app/${game.appid}/guides/?searchText=${q(achievement.trim())}&browsefilter=trend`,
    },
    {
      label: "Recherche Google",
      hint: "Recherche du jeu et du succès",
      url: `https://www.google.com/search?q=${q(query)}`,
    },
  ];
}

export function openHelp(url: string) {
  // Hors de Tauri (interface simulée), l'ouverture passe par le navigateur
  openUrl(url).catch(() => window.open(url, "_blank", "noopener"));
}
