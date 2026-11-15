// Position d'un menu déroulant sous son bouton, en coordonnées de la fenêtre (position: fixed).
// Le menu échappe ainsi aux cartes qui coupent leur contenu (overflow) dans la vue Tableau.
// Attention : aucun ancêtre du menu ne doit avoir de transform, filter ou backdrop-filter (ex. `backdrop-blur`),
// sinon `fixed` se rapporte à cet ancêtre ; le menu déborde alors d'un parent `overflow-hidden`, et la mise au
// point de son premier élément fait défiler ce parent (la bannière du jeu disparaissait ainsi).

//
// `height` : hauteur réelle du menu, mesurée une fois affiché (0 tant qu'elle est inconnue : 260 px supposés).
// Le menu s'ouvre vers le bas s'il y tient, sinon vers le haut s'il y tient, sinon du côté le plus grand, et
// défile alors (`max-height`). Avant, 260 px étaient toujours supposés : un menu plus long (sections du
// « Déplacer vers… ») sortait de la fenêtre.
export function popoverStyle(
  anchor: HTMLElement | undefined,
  width: number,
  align: "left" | "right" = "left",
  height = 0,
): string {
  if (!anchor) return "";
  const r = anchor.getBoundingClientRect();
  const margin = 8;
  const gap = 4;
  const raw = align === "right" ? r.right - width : r.left;
  const left = Math.max(margin, Math.min(raw, window.innerWidth - width - margin));
  const need = height || 260;
  const below = window.innerHeight - r.bottom - gap - margin;
  const above = r.top - gap - margin;
  const down = below >= need || (above < need && below >= above);
  const vertical = down ? `top: ${r.bottom + gap}px` : `bottom: ${window.innerHeight - r.top + gap}px`;
  const room = Math.max(120, down ? below : above);
  return `position: fixed; left: ${left}px; ${vertical}; width: ${width}px; max-height: ${room}px; overflow-y: auto`;
}
