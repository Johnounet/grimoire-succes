// Thèmes : un sombre (Nuit) et un clair (Papier). Le joueur choisit l'apparence : selon Windows (sombre ou
// clair suivant le réglage du système), toujours sombre, toujours claire. Les réglages `theme_dark` et
// `theme_light` restent lus : un ancien thème retiré (Charbon, Sable…) retombe sur Nuit ou Papier.
//
// Un thème est un jeu de valeurs pour les couleurs de l'interface (`--color-*` de app.css), posées sur <html>.
// Toutes les couleurs de l'interface passent par ces variables : ne jamais écrire une couleur en dur.
// Le dernier thème appliqué est gardé dans le navigateur (`localStorage`) pour être posé avant le premier
// affichage (`applyCachedTheme`, appelé par main.ts) : pas d'éclair de thème sombre au démarrage.
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, type SectionColor } from "./library";
import { send } from "./sync";

export interface Palette {
  /** Fond de la fenêtre */
  bg: string;
  /** Panneaux et cartes */
  panel: string;
  /** Éléments en relief (boutons, champs, survol) */
  raised: string;
  /** Bordures */
  line: string;
  /** Texte */
  fg: string;
  /** Texte secondaire */
  muted: string;
  accent: string;
  /** Texte sur un bouton d'accent */
  "on-accent": string;
  /** 100 %, succès ultra rares */
  gold: string;
  /** Succès rares */
  rare: string;
  /** Obtenu, réussi */
  ok: string;
  danger: string;
}

export type ThemeId = "nuit" | "papier";

export interface Theme {
  id: ThemeId;
  name: string;
  hint: string;
  dark: boolean;
  colors: Palette;
  /** Couleurs des sections de l'espace de travail (`--section-<nom>`), choisies pour aller ensemble
   *  et rester lisibles sur le fond du thème. */
  sections: Record<SectionColor, string>;
}

export const themes: Theme[] = [
  {
    id: "nuit",
    name: "Nuit",
    hint: "Bleu-gris, accent bleu",
    dark: true,
    colors: {
      bg: "#0f1116",
      panel: "#161922",
      raised: "#1e222d",
      line: "#2a2f3c",
      fg: "#e6e8ee",
      muted: "#8b93a7",
      accent: "#4f7cff",
      "on-accent": "#ffffff",
      gold: "#f2c14e",
      rare: "#b07cff",
      ok: "#4cc38a",
      danger: "#ef5b5b",
    },
    sections: {
      violet: "#9b8cff",
      bleu: "#5b9dff",
      turquoise: "#3cc8b4",
      vert: "#6cc56b",
      ambre: "#e5a93d",
      corail: "#ec7a5c",
      rose: "#e86fae",
      gris: "#9aa3b5",
    },
  },
  {
    id: "papier",
    name: "Papier",
    hint: "Blanc neutre, accent bleu",
    dark: false,
    colors: {
      bg: "#f3f5f8",
      panel: "#ffffff",
      raised: "#eaedf2",
      line: "#d6dbe4",
      fg: "#1b1f29",
      muted: "#5b6375",
      accent: "#2f5fe0",
      "on-accent": "#ffffff",
      gold: "#a8740a",
      rare: "#7b45d6",
      ok: "#1d8a55",
      danger: "#c93434",
    },
    sections: {
      violet: "#6a52d9",
      bleu: "#2a6fd6",
      turquoise: "#0f8a7a",
      vert: "#3f8a2e",
      ambre: "#a86f06",
      corail: "#c4502f",
      rose: "#b8327a",
      gris: "#6b7385",
    },
  },
];

export type AppearanceMode = "system" | "dark" | "light";

const query = typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: dark)") : null;

export const appearance = $state({
  mode: "system" as AppearanceMode,
  dark: "nuit" as ThemeId,
  light: "papier" as ThemeId,
  /** Windows est en mode sombre. */
  systemDark: query?.matches ?? true,
});
query?.addEventListener("change", (e) => (appearance.systemDark = e.matches));

const byId = (id: string | null, fallback: ThemeId, dark: boolean): ThemeId => {
  const t = themes.find((x) => x.id === id);
  return t && t.dark === dark ? t.id : fallback;
};

/** Thème affiché en ce moment. */
export function currentTheme(): Theme {
  const dark = appearance.mode === "dark" || (appearance.mode === "system" && appearance.systemDark);
  const id = dark ? appearance.dark : appearance.light;
  return themes.find((t) => t.id === id)!;
}

export async function loadAppearance() {
  const [mode, dark, light] = await Promise.all([
    api.getSetting("appearance"),
    api.getSetting("theme_dark"),
    api.getSetting("theme_light"),
  ]).catch(() => [null, null, null]);
  appearance.mode = mode === "dark" || mode === "light" ? mode : "system";
  appearance.dark = byId(dark, "nuit", true);
  appearance.light = byId(light, "papier", false);
}

export async function setAppearance(patch: Partial<Pick<typeof appearance, "mode" | "dark" | "light">>) {
  Object.assign(appearance, patch);
  const { mode, dark, light } = appearance;
  send("theme-changed", { mode, dark, light });
  await Promise.all([
    api.setSetting("appearance", mode),
    api.setSetting("theme_dark", dark),
    api.setSetting("theme_light", light),
  ]);
}

const CACHE_KEY = "theme";

function paint(theme: Theme) {
  const root = document.documentElement;
  for (const [k, v] of Object.entries(theme.colors)) root.style.setProperty(`--color-${k}`, v);
  for (const [k, v] of Object.entries(theme.sections)) root.style.setProperty(`--section-${k}`, v);
  root.style.colorScheme = theme.dark ? "dark" : "light";
  root.dataset.theme = theme.id;
}

/** À appeler dans un $effect de chaque fenêtre : pose le thème courant. */
export function applyTheme() {
  const theme = currentTheme();
  paint(theme);
  try {
    localStorage.setItem(CACHE_KEY, theme.id);
  } catch {
    /* stockage indisponible : le thème sera posé un peu plus tard au démarrage */
  }
  // Barre de titre de Windows dans la même teinte. Attention : le thème imposé à la fenêtre est aussi ce que
  // le moteur web répond à `prefers-color-scheme`. En mode « selon Windows », on rend donc la fenêtre au
  // système (null) au lieu de la forcer, sinon `appearance.systemDark` ne verrait plus le vrai réglage.
  try {
    getCurrentWindow()
      .setTheme(appearance.mode === "system" ? null : theme.dark ? "dark" : "light")
      .then(() => {
        // Relecture une fois la fenêtre rendue au système (au cas où l'événement `change` ne viendrait pas)
        if (appearance.mode === "system" && query) setTimeout(() => (appearance.systemDark = query.matches), 150);
      })
      .catch(() => {});
  } catch {
    /* hors de Tauri */
  }
}

/** Au démarrage, avant le premier affichage : dernier thème utilisé. */
export function applyCachedTheme() {
  try {
    const t = themes.find((x) => x.id === localStorage.getItem(CACHE_KEY));
    if (t) paint(t);
  } catch {
    /* stockage indisponible */
  }
}
