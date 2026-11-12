// Position d'un menu déroulant sous son bouton, en coordonnées de la fenêtre (position: fixed).
// Le menu échappe ainsi aux cartes qui coupent leur contenu (overflow) dans la vue Tableau.

export function popoverStyle(anchor: HTMLElement | undefined, width: number, align: "left" | "right" = "left"): string {
  if (!anchor) return "";
  const r = anchor.getBoundingClientRect();
  const margin = 8;
  const raw = align === "right" ? r.right - width : r.left;
  const left = Math.max(margin, Math.min(raw, window.innerWidth - width - margin));
  // Pas assez de place en dessous : on ouvre vers le haut
  const below = window.innerHeight - r.bottom;
  const vertical = below < 260 && r.top > below ? `bottom: ${window.innerHeight - r.top + 4}px` : `top: ${r.bottom + 4}px`;
  return `position: fixed; left: ${left}px; ${vertical}; width: ${width}px`;
}
