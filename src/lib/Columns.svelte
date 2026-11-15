<script lang="ts" generics="T">
  // Colonnes automatiques, remplies **strictement de gauche à droite** (choix de l'utilisateur, 2026-11-15) :
  // avec n colonnes, l'élément i va dans la colonne i mod n, sous l'élément i − n. L'ordre se lit comme un
  // livre et « Placer avant » donne une place prévisible. (Avant : chaque élément allait dans la colonne la
  // moins remplie ; plus égal en bas, mais l'ordre devenait illisible et l'utilisateur s'y perdait.)
  // Les éléments sont placés en position absolue et ne changent jamais de place dans le DOM : une carte
  // n'est pas recréée quand la disposition change (texte en cours de saisie, vidéo, focus conservés).
  // Une carte qui grandit pousse seulement celles du dessous dans sa colonne.
  import type { Snippet } from "svelte";

  let {
    items,
    key,
    children,
    minWidth = 360,
    gap = 10,
  }: {
    items: T[];
    key: (it: T) => number;
    children: Snippet<[T]>;
    /** Largeur minimale d'une colonne (px). */
    minWidth?: number;
    gap?: number;
  } = $props();

  let width = $state(0);
  const cols = $derived(Math.max(1, Math.floor((width + gap) / (minWidth + gap))));
  const colWidth = $derived(cols > 0 ? (width - gap * (cols - 1)) / cols : width);

  /** Hauteur mesurée de chaque élément. */
  const heights: Record<number, number> = $state({});

  /** Position de chaque élément, et hauteur totale. */
  const layout = $derived.by(() => {
    const h = Array<number>(cols).fill(0);
    const pos: Record<number, { left: number; top: number }> = {};
    items.forEach((it, i) => {
      const k = key(it);
      const c = i % cols;
      pos[k] = { left: c * (colWidth + gap), top: h[c] };
      h[c] += (heights[k] ?? 60) + gap;
    });
    return { pos, height: Math.max(0, Math.max(...h) - gap) };
  });
</script>

<div bind:clientWidth={width} class="relative" style:height="{layout.height}px">
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
</div>
