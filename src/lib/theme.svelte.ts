// Thèmes : 4 sombres et 4 clairs. Le joueur choisit son thème préféré dans chaque famille, et
// l'apparence : selon Windows (sombre ou clair suivant le réglage du système), toujours sombre, toujours claire.
//
// Un thème est un jeu de valeurs pour les couleurs de l'interface (`--color-*` de app.css), posées sur <html>.
// Toutes les couleurs de l'interface passent par ces variables : ne jamais écrire une couleur en dur.
// Le dernier thème appliqué est gardé dans le navigateur (`localStorage`) pour être posé avant le premier
// affichage (`applyCachedTheme`, appelé par main.ts) : pas d'éclair de thème sombre au démarrage.
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api } from "./library";
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

export type ThemeId = "nuit" | "charbon" | "crepuscule" | "braise" | "papier" | "sable" | "menthe" | "lavande";

export interface Theme {
  id: ThemeId;
  name: string;
  hint: string;
  dark: boolean;
  colors: Palette;
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
  },
  {
    id: "charbon",
    name: "Charbon",
    hint: "Noir profond, accent turquoise",
    dark: true,
    colors: {
      bg: "#000000",
      panel: "#0b0b0b",
      raised: "#171717",
      line: "#2a2a2a",
      fg: "#f2f2f2",
      muted: "#8f8f8f",
      accent: "#2dd4bf",
      "on-accent": "#04231e",
      gold: "#f5c84c",
      rare: "#c08bff",
      ok: "#4ade80",
      danger: "#f87171",
    },
  },
  {
    id: "crepuscule",
    name: "Crépuscule",
    hint: "Violet profond, accent rose",
    dark: true,
    colors: {
      bg: "#15101f",
      panel: "#1d1629",
      raised: "#281e38",
      line: "#3a2d50",
      fg: "#ece6f7",
      muted: "#a193bd",
      accent: "#e05aa8",
      "on-accent": "#ffffff",
      gold: "#f4c55a",
      rare: "#9fb4ff",
      ok: "#5fd39a",
      danger: "#ff6b7a",
    },
  },
  {
    id: "braise",
    name: "Braise",
    hint: "Brun chaud, accent orange",
    dark: true,
    colors: {
      bg: "#17120e",
      panel: "#211a14",
      raised: "#2c231b",
      line: "#3f3328",
      fg: "#f2e9df",
      muted: "#ab998c",
      accent: "#f08a3c",
      "on-accent": "#2a1405",
      gold: "#f6cf5a",
      rare: "#c49bff",
      ok: "#8fcf6a",
      danger: "#ef6a5b",
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
  },
  {
    id: "sable",
    name: "Sable",
    hint: "Beige chaud, accent terracotta",
    dark: false,
    colors: {
      bg: "#f3ecdf",
      panel: "#fbf7f0",
      raised: "#ebe1cf",
      line: "#d8c9b1",
      fg: "#2e261c",
      muted: "#6d5f4e",
      accent: "#b84d28",
      "on-accent": "#ffffff",
      gold: "#9a6a05",
      rare: "#7442c4",
      ok: "#457a24",
      danger: "#b8322a",
    },
  },
  {
    id: "menthe",
    name: "Menthe",
    hint: "Vert pâle, accent vert sapin",
    dark: false,
    colors: {
      bg: "#ebf5ef",
      panel: "#f8fcf9",
      raised: "#dcece3",
      line: "#c0d9ca",
      fg: "#16281e",
      muted: "#527062",
      accent: "#1a7a50",
      "on-accent": "#ffffff",
      gold: "#9a6a05",
      rare: "#7442c4",
      ok: "#1a7a50",
      danger: "#c03636",
    },
  },
  {
    id: "lavande",
    name: "Lavande",
    hint: "Lilas, accent violet",
    dark: false,
    colors: {
      bg: "#f1edf9",
      panel: "#fbf9fe",
      raised: "#e6dff5",
      line: "#d1c6e8",
      fg: "#241d36",
      muted: "#645a80",
      accent: "#6d43cc",
      "on-accent": "#ffffff",
      gold: "#9a6a05",
      rare: "#b0306f",
      ok: "#2a8a57",
      danger: "#c2334a",
    },
  },
];

export const darkThemes = themes.filter((t) => t.dark);
export const lightThemes = themes.filter((t) => !t.dark);

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

/** Variables CSS d'une palette, pour un attribut `style` (aperçus des Réglages). */
export const paletteStyle = (p: Palette) =>
  Object.entries(p)
    .map(([k, v]) => `--color-${k}: ${v}`)
    .join("; ");

const CACHE_KEY = "theme";

function paint(theme: Theme) {
  const root = document.documentElement;
  for (const [k, v] of Object.entries(theme.colors)) root.style.setProperty(`--color-${k}`, v);
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
