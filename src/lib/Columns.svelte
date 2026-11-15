<script lang="ts" generics="T">
  // Colonnes choisies par le joueur (2026-11-15) : `columns` colonnes de même largeur, et chaque élément
  // dans la colonne que donne `colOf`, dans l'ordre reçu (de haut en bas). Rien n'est calculé à la place du
  // joueur : un élément ne change de colonne que s'il le déplace. (Essayés avant et abandonnés : colonne la
  // moins remplie, puis remplissage de gauche à droite ; l'utilisateur ne s'y retrouvait pas pour ranger.)
  // Fenêtre trop étroite : les colonnes ne descendent pas sous `minWidth`, la zone défile sur le côté.
  // Les éléments sont placés en position absolue et ne changent jamais de place dans le DOM : une carte
  // n'est pas recréée quand elle change de colonne (texte en cours, vidéo, focus conservés).
  // `footer` : contenu en bas de chaque colonne (cible « Mettre ici » pendant un déplacement).
  import type { Snippet } from "svelte";

  let {
    items,
    key,
    columns,
    colOf,
    children,
    footer,
    minWidth = 240,
    gap = 10,
  }: {
    items: T[];
    key: (it: T) => number;
    /** Nombre de colonnes. */
    columns: number;
    /** Colonne d'un élément (bornée à 0..columns − 1). */
    colOf: (it: T) => number;
    children: Snippet<[T]>;
    footer?: Snippet<[number]>;
    /** Largeur minimale d'une colonne (px). */
    minWidth?: number;
    gap?: number;
  } = $props();

  /** Hauteur réservée au contenu de `footer`. */
  const FOOTER = 44;

  let width = $state(0);
  const n = $derived(Math.max(1, columns));
  const colWidth = $derived(Math.max(minWidth, (width - gap * (n - 1)) / n));
  const inner = $derived(n * colWidth + gap * (n - 1));
  const scroll = $derived(inner > width + 1);

  /** Hauteur mesurée de chaque élément. */
  const heights: Record<number, number> = $state({});

  /** Position de chaque élément, haut du pied de chaque colonne, et hauteur totale. */
  const layout = $derived.by(() => {
    const h = Array<number>(n).fill(0);
    const pos: Record<number, { left: number; top: number }> = {};
    for (const it of items) {
      const k = key(it);
      const c = Math.max(0, Math.min(n - 1, colOf(it)));
      pos[k] = { left: c * (colWidth + gap), top: h[c] };
      h[c] += (heights[k] ?? 60) + gap;
    }
    const feet = [...h];
    const total = footer ? Math.max(...h) + FOOTER : Math.max(0, Math.max(...h) - gap);
    return { pos, feet, height: total };
  });
</script>

<div bind:clientWidth={width} class={scroll ? "overflow-x-auto pb-1" : ""}>
  <div class="relative" style:height="{layout.height}px" style:width={scroll ? `${inner}px` : undefined}>
    {#each items as it (key(it))}
      {@const k = key(it)}
      <div
        bind:clientHeight={heights[k]}
        class="absolute"
        style:left="{layout.pos[k]?.left ?? 0}px"
        style:top="{layout.pos[k]?.top ?? 0}px"
        style:width="{colWidth}px"
      >
        {@render children(it)}
      </div>
    {/each}
    {#if footer}
      {#each layout.feet as top, c (c)}
        <div class="absolute" style:left="{c * (colWidth + gap)}px" style:top="{top}px" style:width="{colWidth}px" style:height="{FOOTER - gap}px">
          {@render footer(c)}
        </div>
      {/each}
    {/if}
  </div>
</div>
