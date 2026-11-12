// Ouverture de la fenêtre compacte (fenêtre Tauri « compact », toujours au premier plan).
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { send } from "./sync";

export async function openCompact(appid: number) {
  try {
    const existing = await WebviewWindow.getByLabel("compact");
    if (existing) {
      await existing.setFocus();
      send("compact-game", { appid });
      return;
    }
    // Même page que la fenêtre principale : main.ts affiche la vue compacte selon le nom de la fenêtre.
    // Le jeu ouvert lui est envoyé quand elle annonce qu'elle est prête (« compact-ready »).
    new WebviewWindow("compact", {
      url: "index.html",
      title: "Chasse aux succès · compact",
      width: 340,
      height: 540,
      minWidth: 260,
      minHeight: 160,
      alwaysOnTop: true,
      decorations: false,
      transparent: true,
      resizable: true,
    });
  } catch {
    // Interface simulée dans un navigateur : simple onglet
    window.open("/?compact", "compact", "width=360,height=560");
  }
}
