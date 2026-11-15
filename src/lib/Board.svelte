<script lang="ts">
  // Vue Tableau : les cartes sur une grille aimantée de 12 colonnes (gridstack).
  // Chaque carte se déplace (poignée ou titre) et se redimensionne (bords et coins) ;
  // elles ne se chevauchent jamais, les voisines se poussent pour faire de la place.
  //
  // Svelte rend le contenu des cartes ; gridstack ne fait que les positionner.
  // Une carte sans position (ajoutée depuis la liste, ou ancienne) reçoit une place libre.
  // Une carte grandit (en hauteur, jamais ne rapetisse) quand le joueur y ajoute du contenu
  // (`watchGrowth`), et quand elle arrive dans l'espace avec des blocs (`fresh`).
  // La grille reste libre (`float`) : rien ne remonte tout seul, sauf les cartes situées sous une carte
  // que le joueur retire (`willRemove`), et toutes d'un coup avec « Tout remonter » (`packAll`).
  //
  // Sections : un tableau par section (Workspace.svelte). Une carte glissée d'un tableau à l'autre est
  // déplacée par gridstack dans le DOM ; on la rend aussitôt à son tableau d'origine (que Svelte la
  // retire lui-même) et on signale le déplacement (`onmovein`) : Svelte recrée la carte dans la section.
  import { GridStack, type GridItemHTMLElement, type GridStackNode } from "gridstack";
  import "gridstack/dist/gridstack.min.css";
  import { tick, untrack, type Snippet } from "svelte";
  import { cardKey, cardSizes, DRAG_MIME, FIT_CARD_EVENT, type CardKey, type GridPos, type WorkspaceItem } from "./library";
  import { arrive, reduced } from "./motion.svelte";
  import { ui } from "./settings.svelte";

  const DEFAULT = { w: 4, h: 8 };
  let {
    items,
    card,
    onlayout,
    ondropids,
    onmovein,
    ondragcard,
    framed = false,
    minRow = 8,
    spare = DEFAULT.h + 2,
    hint = "",
  }: {
    items: WorkspaceItem[];
    card: Snippet<[WorkspaceItem]>;
    /** Carte venue d'un autre tableau (autre section), déposée à cette place. */
    onmovein: (key: CardKey, pos: GridPos) => void;
    /** Une carte de ce tableau commence (true) ou finit (false) d'être déplacée. */
    ondragcard?: (dragging: boolean) => void;
    /** Tableau dans un cadre (section) : c'est le cadre qui s'éclaire à l'arrivée d'une carte. */
    framed?: boolean;
    /** Hauteur minimale de la grille, en rangées. */
    minRow?: number;
    /** Rangées libres gardées sous les cartes, pour en déposer de nouvelles. */
    spare?: number;
    /** Texte affiché quand le tableau est vide. */
    hint?: string;
    /** Nouvelles positions après un déplacement ou un redimensionnement, par `cardKey`. */
    onlayout: (positions: Map<CardKey, GridPos>) => void;
    /** Succès glissés depuis le panneau et déposés sur la case (x, y). */
    ondropids: (ids: number[], at: { x: number; y: number }) => void;
  } = $props();

  type Origin = { parent: Node; next: Node | null };
  /** Place d'origine dans le DOM d'une carte en cours de déplacement (pour l'y rendre si elle change de tableau). */
  const ORIGIN = Symbol("origin");
  let grid: GridStack | null = null;
  let hover = $state(false);

  /** Hauteur minimale d'une carte, en lignes de la grille. */
  const MIN_H = 3;

  /** Cartes que le joueur retire : leur place sera comblée par les cartes du dessous. */
  const closing = new Set<CardKey>();

  export function willRemove(id: CardKey) {
    closing.add(id);
  }

  /** Remonte toutes les cartes au plus haut, chacune dans sa colonne (aucune ne change de côté). */
  export function packAll() {
    slideUp(grid?.engine.nodes ?? [], 0);
  }

  type Area = { x: number; y: number; w: number; h: number };
  const area = (n: GridStackNode): Area => ({ x: n.x ?? 0, y: n.y ?? 0, w: n.w ?? 1, h: n.h ?? 1 });
  const overlap = (a: Area, b: Area) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;

  /** Fait remonter, dans la place libérée, les cartes qui étaient en dessous (sans toucher aux autres). */
  function closeGap(hole: Area) {
    const below = (grid?.engine.nodes ?? []).filter(
      (n) => area(n).y >= hole.y && area(n).x < hole.x + hole.w && hole.x < area(n).x + area(n).w,
    );
    slideUp(below, hole.y);
  }

  /** Remonte ces cartes tant que la place au-dessus est libre, sans dépasser la ligne `floor`. */
  function slideUp(nodes: GridStackNode[], floor: number) {
    const g = grid;
    if (!g) return;
    const free = (n: GridStackNode, y: number) => {
      const want = { ...area(n), y };
      return g.engine.nodes.every((o) => o === n || !overlap(area(o), want));
    };
    // De haut en bas : chaque carte remontée libère la place de la suivante
    const sorted = [...nodes].sort((a, b) => area(a).y - area(b).y);
    g.batchUpdate();
    for (const n of sorted) {
      let y = area(n).y;
      while (y > floor && free(n, y - 1)) y--;
      if (y !== n.y && n.el) g.update(n.el, { y });
    }
    g.batchUpdate(false);
    // gridstack ne signale pas toujours ces déplacements par « change » : on enregistre nous-mêmes
    emitLayout();
  }

  function emitLayout() {
    if (!grid) return;
    const positions = new Map<CardKey, GridPos>();
    for (const n of grid.engine.nodes) {
      const id = Number(n.el?.dataset.id);
      if (Number.isFinite(id)) positions.set(id, { x: n.x ?? 0, y: n.y ?? 0, w: n.w ?? DEFAULT.w, h: n.h ?? DEFAULT.h });
    }
    onlayout(positions);
  }

  /** Zone qui défile autour de la grille (la plus proche), sinon la page. */
  function scrollParent(el: HTMLElement): HTMLElement {
    for (let p = el.parentElement; p; p = p.parentElement) {
      const o = getComputedStyle(p).overflowY;
      if ((o === "auto" || o === "scroll") && p.scrollHeight > p.clientHeight) return p;
    }
    return (document.scrollingElement as HTMLElement | null) ?? document.documentElement;
  }

  /** Bande (px) près du bord haut ou bas où la souris fait défiler, et vitesse maximale (px par image). */
  const EDGE = 60;
  const EDGE_SPEED = 20;

  /**
   * Pendant le glisser d'une carte : défile seulement quand la souris est près du bord haut ou bas de la
   * zone qui défile, d'autant plus vite qu'elle en est proche. Jusqu'au relâchement du bouton.
   */
  function edgeScroll(gridEl: HTMLElement) {
    const box = scrollParent(gridEl);
    const isPage = box === document.scrollingElement || box === document.documentElement;
    let x = 0;
    let y = -1;
    let raf = 0;
    const ours = new WeakSet<Event>();
    const speed = () => {
      const r = isPage ? { top: 0, bottom: window.innerHeight } : box.getBoundingClientRect();
      const top = Math.max(r.top, 0);
      const bottom = Math.min(r.bottom, window.innerHeight);
      if (y < 0) return 0;
      if (y < top + EDGE) return -Math.min(1, (top + EDGE - y) / EDGE);
      if (y > bottom - EDGE) return Math.min(1, (y - (bottom - EDGE)) / EDGE);
      return 0;
    };
    const step = () => {
      raf = 0;
      const v = speed();
      if (!v) return;
      const before = box.scrollTop;
      box.scrollTop += Math.round(v * EDGE_SPEED) || Math.sign(v);
      if (box.scrollTop === before) return; // déjà tout en haut ou tout en bas
      // La grille a bougé sous la souris immobile : gridstack recalcule la place de la carte
      const ev = new MouseEvent("mousemove", { clientX: x, clientY: y, bubbles: true });
      ours.add(ev);
      document.dispatchEvent(ev);
      raf = requestAnimationFrame(step);
    };
    const move = (e: MouseEvent) => {
      if (ours.has(e)) return; // le nôtre, envoyé par `step`
      x = e.clientX;
      y = e.clientY;
      if (!raf) raf = requestAnimationFrame(step);
    };
    const stop = () => {
      cancelAnimationFrame(raf);
      document.removeEventListener("mousemove", move, true);
    };
    document.addEventListener("mousemove", move, true);
    document.addEventListener("mouseup", stop, { once: true, capture: true });
  }

  /** Conteneur : crée la grille (qui adopte les cartes déjà présentes). */
  function setupGrid(el: HTMLElement) {
    const g = GridStack.init(
      {
        column: 12,
        cellHeight: 40,
        margin: 6,
        float: true,
        animate: untrack(() => !reduced()), // sans dépendance : la grille ne doit pas être recréée
        minRow: untrack(() => minRow),
        acceptWidgets: true,
        handle: ".card-drag",
        // Pas le défilement de gridstack : il fait défiler dès qu'un bout de la carte glissée sort de
        // l'écran (presque toujours pour une grande carte), d'où un écran qui bouge au moindre geste.
        // Remplacé par `edgeScroll` : seulement quand la souris approche du bord.
        draggable: { handle: ".card-drag", scroll: false },
        resizable: { handles: "e, se, s, sw, w" },
      },
      el,
    );
    if (!g) return;
    grid = g;
    // Réglage des animations changé pendant que le tableau est affiché
    const stopMotion = $effect.root(() => {
      $effect(() => {
        g.setAnimation(!reduced());
      });
    });
    // Une seule notification par geste : fin de déplacement / redimensionnement, ajout
    g.on("change", emitLayout);
    g.on("added", emitLayout);
    g.on("dragstart", (_e: Event, target: HTMLElement) => {
      // `target` peut être la poignée : on note la place de la carte elle-même
      const el = target.closest<HTMLElement & { [ORIGIN]?: Origin }>(".grid-stack-item");
      if (el?.parentNode) el[ORIGIN] = { parent: el.parentNode, next: el.nextSibling };
      edgeScroll(g.el);
      // Fin du geste : au relâchement (gridstack ne prévient pas la grille de départ si la carte en change)
      if (ondragcard) {
        ondragcard(true);
        document.addEventListener("mouseup", () => setTimeout(() => ondragcard(false)), { once: true, capture: true });
      }
    });
    g.on("dropped", (_e: Event, prev: GridStackNode | undefined, node: GridStackNode) => {
      const el = node.el as (GridItemHTMLElement & { [ORIGIN]?: Origin }) | undefined;
      if (!el || !prev) return;
      const origin = el[ORIGIN] ?? (prev.grid?.el ? { parent: prev.grid.el, next: null } : undefined);
      delete el[ORIGIN];
      if (!origin) return;
      const key = Number(el.dataset.id);
      const pos = { x: node.x ?? 0, y: node.y ?? 0, w: node.w ?? DEFAULT.w, h: node.h ?? DEFAULT.h };
      // La carte appartient à Svelte : retirée de cette grille et rendue à sa place d'origine,
      // cachée jusqu'à ce que Svelte la supprime et la recrée ici
      g.removeWidget(el, false, false);
      el.style.display = "none";
      origin.parent.insertBefore(el, origin.next && origin.next.parentNode === origin.parent ? origin.next : null);
      if (Number.isFinite(key)) onmovein(key, pos);
    });
    const onFit = (e: Event) => {
      const item = (e.target as HTMLElement).closest(".grid-stack-item");
      if (!item) return;
      if ((e as CustomEvent<{ shrink?: boolean } | null>).detail?.shrink) shrinkCard(item as GridItemHTMLElement);
      else fitCard(item as GridItemHTMLElement);
    };
    el.addEventListener(FIT_CARD_EVENT, onFit);
    return () => {
      stopMotion();
      el.removeEventListener(FIT_CARD_EVENT, onFit);
      grid?.destroy(false);
      grid = null;
    };
  }

  /**
   * Agrandit la carte (en hauteur seulement) pour que tout son contenu soit visible sans défiler.
   * Une carte déjà assez grande ne bouge pas ; les cartes du dessous sont poussées si besoin.
   */
  function fitCard(el: GridItemHTMLElement) {
    const n = el.gridstackNode;
    const rows = neededRows(el);
    if (!grid || !n || rows == null) return;
    if (rows > (n.h ?? 0)) {
      grid.update(el, { h: rows });
      emitLayout();
    }
  }

  /**
   * Bloc retiré ou replié par le joueur : la carte se cale juste sous ce qu'elle contient encore,
   * et les cartes du dessous remontent dans la place libérée (sans changer de colonne).
   * Une carte déjà plus petite que son contenu (réduite exprès) ne bouge pas.
   */
  function shrinkCard(el: GridItemHTMLElement) {
    const n = el.gridstackNode;
    const rows = neededRows(el);
    if (!grid || !n || rows == null) return;
    const before = area(n);
    const h = Math.max(MIN_H, rows);
    if (h >= before.h) return;
    grid.update(el, { h });
    closeGap({ x: before.x, y: before.y + h, w: before.w, h: before.h - h }); // enregistre aussi
  }

  /** Nombre de lignes de la grille qu'il faut à la carte pour tout montrer sans défiler. */
  function neededRows(el: GridItemHTMLElement): number | null {
    const card = el.querySelector("article");
    if (!grid || !card) return null;
    // Hauteur naturelle : en-tête + contenu (et marges de la zone qui défile) + bordures
    const [header, body] = Array.from(card.children) as HTMLElement[];
    const content = body?.querySelector<HTMLElement>("[data-card-content]");
    let bodyH = body?.scrollHeight ?? 0;
    if (body && content) {
      const st = getComputedStyle(body);
      bodyH = content.offsetHeight + parseFloat(st.paddingTop) + parseFloat(st.paddingBottom) + parseFloat(st.borderTopWidth);
    }
    const borders = card.offsetHeight - card.clientHeight;
    const need = (header?.offsetHeight ?? 0) + bodyH + borders;
    const margin = 6; // marge de gridstack, au-dessus et au-dessous du contenu
    return Math.ceil((need + 2 * margin) / grid.getCellHeight(true));
  }

  /**
   * Agrandit la carte quand son contenu grandit à la suite d'une action du joueur dans la carte
   * (bloc ajouté, élément de liste, texte, image, vidéo…).
   * Les changements sans action récente (chargement, images qui arrivent) ne comptent pas : une carte
   * que le joueur a volontairement réduite le reste. Un redimensionnement de la carte non plus
   * (la largeur change, le texte se replie).
   */
  function watchGrowth(el: GridItemHTMLElement): () => void {
    const content = el.querySelector<HTMLElement>("[data-card-content]");
    if (!content || typeof ResizeObserver === "undefined") return () => {};
    let lastAction = 0;
    const acted = () => (lastAction = Date.now());
    const events = ["pointerdown", "keydown", "input", "paste", "drop"] as const;
    for (const ev of events) el.addEventListener(ev, acted, true);
    // Largeur de la carte elle-même : celle du contenu varie quand la barre de défilement apparaît
    let width = el.offsetWidth;
    let height = content.offsetHeight;
    const ro = new ResizeObserver(() => {
      const w = el.offsetWidth;
      const h = content.offsetHeight;
      const grew = h > height && w === width;
      width = w;
      height = h;
      const busy = el.classList.contains("ui-resizable-resizing") || el.classList.contains("ui-draggable-dragging");
      // 3 s : laisse le temps à une image envoyée ou une vidéo d'apparaître
      if (grew && !busy && Date.now() - lastAction < 3000) fitCard(el);
    });
    ro.observe(content);
    return () => {
      ro.disconnect();
      for (const ev of events) el.removeEventListener(ev, acted, true);
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
        const size = item.size ?? DEFAULT;
        el.setAttribute("gs-w", String(size.w));
        el.setAttribute("gs-h", String(size.h));
        el.setAttribute("gs-auto-position", "true");
      }
      el.setAttribute("gs-min-w", "2");
      el.setAttribute("gs-min-h", String(MIN_H));
      // Grille déjà créée : carte ajoutée après coup. Sinon, GridStack.init l'adoptera.
      queueMicrotask(() => {
        if (grid && !node.gridstackNode) grid.makeWidget(el);
        // Carte qui vient d'arriver : agrandie si ses blocs ne tiennent pas dedans (une fois affichés)
        if (item.fresh) requestAnimationFrame(() => fitCard(node));
      });
      const stopWatch = watchGrowth(node);
      const id = cardKey(item);
      return () => {
        stopWatch();
        const n = node.gridstackNode;
        if (!grid || !n) return;
        const hole = area(n);
        grid.removeWidget(el, false);
        // Après la mise à jour de l'affichage : la liste `items` ne contient plus la carte retirée,
        // sinon l'enregistrement des nouvelles positions la ferait revenir
        if (closing.delete(id)) tick().then(() => closeGap(hole));
      };
    });
  }

  // ----- Arrivée depuis le panneau (glisser natif) -----
  // Un aperçu montre où la carte arrivera : la place libre la plus proche du curseur.
  // On ne dépose jamais sur une carte existante (rien n'est poussé) ; le tableau garde
  // toujours de l'espace libre sous les cartes (`DROP_ROWS`) pour en ajouter en bas.
  const DROP_ROWS = $derived(spare);
  /** Taille de la carte qui arrive : sa dernière taille connue, sinon la taille par défaut. */
  const dropSize = () => cardSizes.get(ui.draggingIds[0]) ?? DEFAULT;
  let gridEl: HTMLElement | undefined = $state();
  let ghost = $state<{ x: number; y: number } | null>(null);

  /** Case sous le curseur (coordonnées écran), carte centrée horizontalement. */
  function cellAt(e: DragEvent): { x: number; y: number } | null {
    if (!grid || !gridEl) return null;
    const box = gridEl.getBoundingClientRect();
    const colW = box.width / 12;
    const rowH = grid.getCellHeight(true);
    if (colW <= 0 || rowH <= 0) return null;
    return {
      x: Math.floor((e.clientX - box.left) / colW) - Math.floor(dropSize().w / 2),
      y: Math.max(0, Math.floor((e.clientY - box.top) / rowH)),
    };
  }

  /** Place libre la plus proche de `want` pour la carte qui arrive (`dropSize`). */
  function nearestFree(want: { x: number; y: number }): { x: number; y: number } {
    const g = grid!;
    const size = dropSize();
    const maxX = 12 - size.w;
    const tx = Math.max(0, Math.min(maxX, want.x));
    let best = { x: tx, y: g.getRow() };
    let bestD = Infinity;
    for (let y = 0; y <= g.getRow(); y++) {
      for (let x = 0; x <= maxX; x++) {
        // Distance pondérée : on préfère rester sur la même ligne que monter/descendre
        const d = Math.abs(x - tx) + 1.5 * Math.abs(y - want.y);
        if (d < bestD && g.isAreaEmpty(x, y, size.w, size.h)) {
          bestD = d;
          best = { x, y };
        }
      }
    }
    return best;
  }

  function dragOver(e: DragEvent) {
    if (!e.dataTransfer?.types.includes(DRAG_MIME)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    hover = true;
    const want = cellAt(e);
    if (!want) return;
    const at = nearestFree(want);
    if (ghost?.x !== at.x || ghost?.y !== at.y) ghost = at;
  }
  function dragLeave(e: DragEvent) {
    // Ignore les passages d'un élément enfant à un autre
    if (e.currentTarget instanceof Node && e.relatedTarget instanceof Node && e.currentTarget.contains(e.relatedTarget)) return;
    hover = false;
    ghost = null;
  }
  function drop(e: DragEvent) {
    const at = ghost;
    hover = false;
    ghost = null;
    ui.draggingFromPanel = false;
    ui.draggingIds = [];
    const raw = e.dataTransfer?.getData(DRAG_MIME);
    if (!raw || !grid) return;
    e.preventDefault();
    try {
      const ids = (JSON.parse(raw) as unknown[]).filter((x): x is number => typeof x === "number");
      const want = cellAt(e);
      ondropids(ids, at ?? (want ? nearestFree(want) : { x: 0, y: grid.getRow() }));
    } catch {
      /* données d'un autre glisser : ignorées */
    }
  }
</script>

<div
  role="region"
  aria-label="Tableau des cartes"
  ondragover={dragOver}
  ondragleave={dragLeave}
  ondrop={drop}
  class="board relative flex-1 transition
    {ui.draggingFromPanel ? 'dragging-in' : ''}
    {ui.draggingFromPanel && !framed ? 'rounded-xl outline-2 outline-offset-4 outline-dashed' : ''}
    {framed ? '' : hover ? 'outline-accent bg-accent/5' : 'outline-line'}"
  style:padding-bottom="{DROP_ROWS * 40}px"
>
  {#if hint && items.length === 0 && !ghost}
    <p class="pointer-events-none absolute inset-x-0 top-4 text-center text-sm text-muted">{hint}</p>
  {/if}
  <div class="grid-stack" bind:this={gridEl} {@attach setupGrid}>
    {#each items as item (cardKey(item))}
      <div class="grid-stack-item" data-id={cardKey(item)} {@attach (el) => setupItem(el, () => item)}>
        <div class="grid-stack-item-content" data-card={cardKey(item)} {@attach (el) => arrive(el, cardKey(item))}>
          {@render card(item)}
        </div>
      </div>
    {/each}
  </div>
  {#if ghost}
    <!-- Aperçu de la place d'arrivée -->
    <div
      class="ghost pointer-events-none absolute"
      style:left="calc({ghost.x} * 100% / 12)"
      style:top="{ghost.y * 40}px"
      style:width="calc({dropSize().w} * 100% / 12)"
      style:height="{dropSize().h * 40}px"
    ></div>
  {/if}
</div>

<style>
  /* Pendant une arrivée depuis le panneau, les cartes (et leurs vidéos) ne captent
     pas le glisser : c'est le tableau qui le reçoit, même au-dessus d'une carte. */
  .board.dragging-in :global(.grid-stack-item) {
    pointer-events: none;
  }
  .ghost {
    padding: 6px;
  }
  .ghost::before {
    content: "";
    display: block;
    height: 100%;
    border: 2px dashed var(--zone-color, var(--color-accent));
    border-radius: 4px;
    background: color-mix(in srgb, var(--zone-color, var(--color-accent)) 12%, transparent);
  }
  /* Contenu de la case : la carte la remplit entièrement */
  .board :global(.grid-stack-item-content) {
    overflow: visible !important;
    inset: 0;
  }
  /* Emplacement d'arrivée pendant un déplacement */
  .board :global(.grid-stack-placeholder > .placeholder-content) {
    border: 2px dashed var(--zone-color, var(--color-accent));
    border-radius: 4px;
    background: color-mix(in srgb, var(--zone-color, var(--color-accent)) 12%, transparent);
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
