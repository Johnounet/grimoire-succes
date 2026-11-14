// Ouverture de la fenêtre compacte (fenêtre Tauri « compact », toujours au premier plan).
import { invoke, isTauri } from "@tauri-apps/api/core";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { showToast } from "./blocks/store.svelte";
import { send } from "./sync";

export async function openCompact(appid: number) {
  // Interface simulée dans un navigateur : simple onglet. Jamais dans Tauri : une page ouverte par
  // window.open n'y aurait pas accès au cœur de l'application.
  if (!isTauri()) {
    window.open("/?compact", "compact", "width=360,height=560");
    return;
  }
  try {
    const existing = await WebviewWindow.getByLabel("compact");
    if (existing) {
      // Encore cachée (ouverture en cours, double clic) : elle prendra le premier plan en s'affichant
      if (await existing.isVisible()) await existing.setFocus();
      send("compact-game", { appid });
      return;
    }
    // Créée par le cœur Rust (window_state.rs), cachée puis remise à sa taille et à sa place retenues.
    // Même page que la fenêtre principale : main.ts affiche la vue compacte selon le nom de la fenêtre.
    // Le jeu ouvert lui est envoyé quand elle annonce qu'elle est prête (« compact-ready »).
    await invoke("compact_open");
  } catch (e) {
    showToast(`Mode compact impossible à ouvrir : ${e}`);
  }
}
