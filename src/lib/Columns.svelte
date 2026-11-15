<script lang="ts" generics="T">
  // Colonnes automatiques : chaque élément va dans la colonne la moins remplie, dans l'ordre reçu.
  // Les éléments sont placés en position absolue et ne changent jamais de place dans le DOM : une carte
  // n'est pas recréée quand la disposition change (texte en cours de saisie, vidéo, focus conservés).
  // La répartition entre colonnes est recalculée tout de suite si la liste, l'ordre ou le nombre de
  // colonnes change (ou à la première mesure d'un élément). Quand une carte change de hauteur (bloc
  // ajouté, replié, succès rattaché…), elle est recalculée **une fois le calme revenu** : 500 ms sans
  // nouveau changement, aucune saisie en cours dans les colonnes et aucun glisser. Une carte ne saute donc
  // jamais de colonne pendant qu'on écrit dedans ; le rééquilibrage attend la sortie du champ.
  import { untrack, type Snippet } from "svelte";

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
  const measured = $derived(items.filter((it) => heights[key(it)] != null).length);
  const order = $derived(items.map(key).join(","));

  /** Colonne de chaque élément. */
  let column = $state<Record<number, number>>({});

  function distribute() {
    const n = cols;
    const h = Array<number>(n).fill(0);
    const next: Record<number, number> = {};
    for (const it of items) {
      const k = key(it);
      const c = h.indexOf(Math.min(...h));
      next[k] = c;
      h[c] += (heights[k] ?? 60) + gap;
    }
    if (items.some((it) => next[key(it)] !== column[key(it)])) column = next;
  }

  $effect(() => {
    void order;
    void measured;
    void cols;
    untrack(distribute);
  });

  /** Position de chaque élément, et hauteur totale. */
  const layout = $derived.by(() => {
    const h = Array<number>(cols).fill(0);
    const pos: Record<number, { left: number; top: number }> = {};
    for (const it of items) {
      const k = key(it);
      const c = Math.min(column[k] ?? 0, cols - 1);
      pos[k] = { left: c * (colWidth + gap), top: h[c] };
      h[c] += (heights[k] ?? 60) + gap;
    }
    return { pos, height: Math.max(0, Math.max(...h) - gap) };
  });

  // ----- Rééquilibrage après un changement de hauteur -----
  let box: HTMLDivElement | undefined = $state();
  let dragging = false;
  let pending = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** Saisie en cours dans les colonnes (champ, zone de texte, éditeur). */
  const typing = () => {
    const el = document.activeElement;
    return !!el && !!box?.contains(el) && el.matches("input, textarea, select, [contenteditable]:not([contenteditable=false])");
  };
  function settle() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      if (dragging || typing()) pending = true;
      else {
        pending = false;
        distribute();
      }
    }, 500);
  }
  /** Fin d'une saisie ou d'un glisser : rééquilibrage s'il attendait. */
  function resume() {
    if (pending) settle();
  }
  $effect(() => {
    for (const it of items) void heights[key(it)];
    untrack(settle);
  });
  $effect(() => () => clearTimeout(timer));
</script>

<div
  bind:this={box}
  bind:clientWidth={width}
  class="relative"
  style:height="{layout.height}px"
  onfocusout={() => setTimeout(resume)}
  ondragstart={() => (dragging = true)}
  ondragend={() => {
    dragging = false;
    resume();
  }}
  role="presentation"
>
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
