<script lang="ts">
  // Vue Tableau : les cartes sur une grille aimantée de 12 colonnes (gridstack).
  // Chaque carte se déplace (poignée ou titre) et se redimensionne (bords et coins) ;
  // elles ne se chevauchent jamais, les voisines se poussent pour faire de la place.
  //
  // Svelte rend le contenu des cartes ; gridstack ne fait que les positionner.
  // Une carte sans position (ajoutée depuis la liste, ou ancienne) reçoit une place libre.
  import { GridStack, type GridItemHTMLElement } from "gridstack";
  import "gridstack/dist/gridstack.min.css";
  import { untrack, type Snippet } from "svelte";
  import { DRAG_MIME, type GridPos, type WorkspaceItem } from "./library";
  import { ui } from "./settings.svelte";

  let {
    items,
    card,
    onlayout,
    ondropids,
  }: {
    items: WorkspaceItem[];
    card: Snippet<[WorkspaceItem]>;
    /** Nouvelles positions après un déplacement ou un redimensionnement. */
    onlayout: (positions: Map<number, GridPos>) => void;
    /** Succès glissés depuis le panneau et déposés sur la case (x, y). */
    ondropids: (ids: number[], at: { x: number; y: number }) => void;
  } = $props();

  const DEFAULT = { w: 4, h: 8 };
  let grid: GridStack | null = null;
  let hover = $state(false);

  function emitLayout() {
    if (!grid) return;
    const positions = new Map<number, GridPos>();
    for (const n of grid.engine.nodes) {
      const id = Number(n.el?.dataset.id);
      if (Number.isFinite(id)) positions.set(id, { x: n.x ?? 0, y: n.y ?? 0, w: n.w ?? DEFAULT.w, h: n.h ?? DEFAULT.h });
    }
    onlayout(positions);
  }

  /** Conteneur : crée la grille (qui adopte les cartes déjà présentes). */
  function setupGrid(el: HTMLElement) {
    const g = GridStack.init(
      {
        column: 12,
        cellHeight: 40,
        margin: 6,
        float: true,
        animate: true,
        minRow: 8,
        handle: ".card-drag",
        resizable: { handles: "e, se, s, sw, w" },
      },
      el,
    );
    if (!g) return;
    grid = g;
    // Une seule notification par geste : fin de déplacement / redimensionnement, ajout
    g.on("change", emitLayout);
    g.on("added", emitLayout);
    return () => {
      grid?.destroy(false);
      grid = null;
    };
  }

  /**
   * Carte : position initiale, puis prise en charge par la grille si elle existe déjà.
   * Lancé une seule fois par carte (`untrack`) : ensuite, c'est gridstack qui la déplace.
   * Sinon, chaque enregistrement de position recréerait la carte à son ancienne place.
   */
  function setupItem(el: HTMLElement, getItem: () => WorkspaceItem) {
    return untrack(() => {
      const item = getItem();
      const g = item.grid;
      const node = el as GridItemHTMLElement;
      if (g) {
        el.setAttribute("gs-x", String(g.x));
        el.setAttribute("gs-y", String(g.y));
        el.setAttribute("gs-w", String(g.w));
        el.setAttribute("gs-h", String(g.h));
      } else {
        el.setAttribute("gs-w", String(DEFAULT.w));
        el.setAttribute("gs-h", String(DEFAULT.h));
        el.setAttribute("gs-auto-position", "true");
      }
      el.setAttribute("gs-min-w", "2");
      el.setAttribute("gs-min-h", "3");
      // Grille déjà créée : carte ajoutée après coup. Sinon, GridStack.init l'adoptera.
      queueMicrotask(() => {
        if (grid && !node.gridstackNode) grid.makeWidget(el);
      });
      return () => {
        if (grid && node.gridstackNode) grid.removeWidget(el, false);
      };
    });
  }

  // ----- Arrivée depuis le panneau (glisser natif) -----
  function dragOver(e: DragEvent) {
    if (!e.dataTransfer?.types.includes(DRAG_MIME)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    hover = true;
  }
  function drop(e: DragEvent) {
    hover = false;
    ui.draggingFromPanel = false;
    const raw = e.dataTransfer?.getData(DRAG_MIME);
    if (!raw || !grid) return;
    e.preventDefault();
    try {
      const ids = (JSON.parse(raw) as unknown[]).filter((x): x is number => typeof x === "number");
      const cell = grid.getCellFromPixel({ left: e.clientX, top: e.clientY }, false);
      ondropids(ids, { x: Math.max(0, Math.min(12 - DEFAULT.w, cell.x)), y: Math.max(0, cell.y) });
    } catch {
      /* données d'un autre glisser : ignorées */
    }
  }
</script>

<div
  role="region"
  aria-label="Tableau des cartes"
  ondragover={dragOver}
  ondragleave={() => (hover = false)}
  ondrop={drop}
  class="board rounded-xl transition
    {ui.draggingFromPanel ? 'outline-2 outline-offset-4 outline-dashed' : ''}
    {hover ? 'outline-accent bg-accent/5' : 'outline-line'}"
>
  <div class="grid-stack" {@attach setupGrid}>
    {#each items as item (item.achievement_id)}
      <div class="grid-stack-item" data-id={item.achievement_id} {@attach (el) => setupItem(el, () => item)}>
        <div class="grid-stack-item-content">
          {@render card(item)}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  /* Contenu de la case : la carte la remplit entièrement */
  .board :global(.grid-stack-item-content) {
    overflow: visible !important;
    inset: 0;
  }
  /* Emplacement d'arrivée pendant un déplacement */
  .board :global(.grid-stack-placeholder > .placeholder-content) {
    border: 2px dashed var(--color-accent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
  }
  /* Poignées de redimensionnement discrètes, visibles au survol */
  .board :global(.ui-resizable-handle) {
    opacity: 0;
    transition: opacity 0.15s;
  }
  .board :global(.grid-stack-item:hover > .ui-resizable-handle) {
    opacity: 0.8;
  }
  .board :global(.ui-resizable-se) {
    filter: invert(1);
  }
  /* Carte en cours de déplacement */
  .board :global(.grid-stack-item.ui-draggable-dragging) {
    z-index: 30;
  }
  .board :global(.grid-stack-item.ui-draggable-dragging article) {
    box-shadow: 0 18px 40px -12px rgb(0 0 0 / 0.7);
    border-color: var(--color-accent);
  }
</style>
