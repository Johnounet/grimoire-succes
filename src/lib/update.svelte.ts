// Mises à jour : recherche d'une nouvelle version sur GitHub (au démarrage si le réglage le permet,
// ou à la demande dans « À propos »), installation en un clic pour la version installée, lien vers
// la page de téléchargement pour la version portable.
import { flushBlocks } from "./blocks/store.svelte";
import { api, type UpdateInfo } from "./library";

export type UpdateStatus = "idle" | "checking" | "uptodate" | "available" | "error" | "installing";

export const update = $state({
  status: "idle" as UpdateStatus,
  info: null as UpdateInfo | null,
  error: "",
  /** Bandeau de la nouvelle version fermé (« Plus tard ») pour cette session. */
  dismissed: false,
  /** Recherche au démarrage (réglage `update_check`, activée par défaut). */
  auto: true,
  /** Version portable (`portable.txt` à côté de l'exécutable). */
  portable: false,
});

/** Délai avant la recherche au démarrage : laisse l'application se charger d'abord. */
const STARTUP_DELAY_MS = 5000;

export async function loadUpdatePrefs() {
  const [auto, mode] = await Promise.all([
    api.getSetting("update_check").catch(() => null),
    api.appMode().catch(() => ({ portable: false })),
  ]);
  update.auto = auto !== "0";
  update.portable = mode.portable;
}

export async function setAutoUpdate(on: boolean) {
  update.auto = on;
  await api.setSetting("update_check", on ? "1" : "0");
}

/** Recherche au démarrage, discrète : une erreur (hors ligne, pas encore de version publiée) ne s'affiche pas. */
export function checkOnStartup() {
  setTimeout(() => {
    if (update.auto && update.status === "idle") checkForUpdate(true);
  }, STARTUP_DELAY_MS);
}

export async function checkForUpdate(quiet = false) {
  update.status = "checking";
  update.error = "";
  try {
    update.info = await api.updateCheck();
    update.status = update.info ? "available" : "uptodate";
    update.dismissed = false;
  } catch (e) {
    update.status = quiet ? "idle" : "error";
    update.error = String(e);
  }
}

/** Installe la nouvelle version : blocs enregistrés d'abord, l'application se ferme ensuite. */
export async function installUpdate() {
  update.status = "installing";
  try {
    await flushBlocks();
    await api.updateInstall();
  } catch (e) {
    update.status = "error";
    update.error = String(e);
  }
}

export function openReleasesPage() {
  api.updateOpenPage().catch(() => {});
}
