// Réglages partagés par toute l'interface, enregistrés dans la base via le cœur Rust.
import { api } from "./library";

export const settings = $state({
  /** Floute nom et description des succès cachés non obtenus (activé par défaut). */
  antiSpoiler: true,
});

export async function loadSettings() {
  const v = await api.getSetting("anti_spoiler").catch(() => null);
  settings.antiSpoiler = v !== "0";
}

export async function setAntiSpoiler(on: boolean) {
  settings.antiSpoiler = on;
  await api.setSetting("anti_spoiler", on ? "1" : "0");
}
