<script lang="ts">
  // Blocs d'une carte (succès ou carte libre) : affichage, réordonnancement, ajout, duplication, suppression.
  //
  // Réordonnancement par glisser HTML natif (poignée ⠿) et boutons ↑ ↓ au clavier.
  // Pas svelte-dnd-action ici : ses poignées partagent un état global, et une poignée de bloc
  // rendrait aussi déplaçable la carte qui le contient (zones imbriquées).
  import { dur, pop } from "../motion.svelte";
  import { flip } from "svelte/animate";
  import CounterBlock from "./CounterBlock.svelte";
  import ImageBlock from "./ImageBlock.svelte";
  import ListBlock from "./ListBlock.svelte";
  import { blocksOf, createBlock, deleteBlock, duplicateBlock, reorderBlocks } from "./store.svelte";
  import TextBlock from "./TextBlock.svelte";
  import TimerBlock from "./TimerBlock.svelte";
  import { tick } from "svelte";
  import { CARD_MENU_EVENT, FIT_CARD_EVENT, type CardKey } from "../library";
  import { blockKinds, kindInfo, type Block, type BlockKind } from "./types";
  import VideoBlock from "./VideoBlock.svelte";

  /** `cardKey` : carte d'un succès ou carte libre. */
  let { cardKey, imagesDir }: { cardKey: CardKey; imagesDir: string } = $props();

  const FLIP = 150;
  const MIME = "application/x-chasse-succes-block";
  let menuOpen = $state(false);
  let menuWrap = $state<HTMLElement>();

  // Prévient le tableau (après la mise à jour de l'affichage) : la carte grandit pour montrer le menu,
  // puis reprend sa taille à la fermeture si rien n'a été ajouté
  let menuWasOpen = false;
  $effect(() => {
    const open = menuOpen;
    if (open === menuWasOpen) return;
    menuWasOpen = open;
    menuWrap?.dispatchEvent(new CustomEvent(CARD_MENU_EVENT, { bubbles: true, detail: { open } }));
  });

  const list = $derived(blocksOf(cardKey));

  /** Bloc dont la poignée est enfoncée (seul lui devient « draggable »). */
  let armed = $state<number | null>(null);
  let dragging = $state<number | null>(null);
  /** Position d'insertion pendant le glisser (0 = avant le premier bloc). */
  let dropAt = $state<number | null>(null);

  function move(from: number, to: number) {
    if (from === to || from + 1 === to) return;
    const next = [...list];
    const [b] = next.splice(from, 1);
    next.splice(to > from ? to - 1 : to, 0, b);
    reorderBlocks(cardKey, next);
  }

  function dragStart(e: DragEvent, b: Block) {
    if (armed !== b.id || !e.dataTransfer) return e.preventDefault();
    e.stopPropagation();
    e.dataTransfer.setData(MIME, String(b.id));
    e.dataTransfer.effectAllowed = "move";
    dragging = b.id;
  }

  function dragOver(e: DragEvent, i: number) {
    if (dragging == null || !e.dataTransfer?.types.includes(MIME)) return;
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer.dropEffect = "move";
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    dropAt = e.clientY < r.top + r.height / 2 ? i : i + 1;
  }

  function drop(e: DragEvent) {
    if (dragging == null || dropAt == null) return;
    e.preventDefault();
    e.stopPropagation();
    const from = list.findIndex((b) => b.id === dragging);
    if (from >= 0) move(from, dropAt);
    dragEnd();
  }

  function dragEnd() {
    armed = null;
    dragging = null;
    dropAt = null;
  }

  async function add(kind: BlockKind) {
    menuOpen = false;
    await createBlock(cardKey, kind);
    // Tableau : la carte, revenue à sa taille d'avant le menu, grandit pour montrer le nouveau bloc
    await tick();
    menuWrap?.dispatchEvent(new CustomEvent(FIT_CARD_EVENT, { bubbles: true }));
  }

  // Raccourcis typés pour les composants de chaque bloc
  const as = <K extends BlockKind>(b: Block, _k: K) => b as Block<K>;
</script>

<div class="space-y-2">
  {#if list.length}
    <div class="space-y-2" role="list">
      {#each list as b, i (b.id)}
        {@const info = kindInfo(b.kind)}
        <section
          animate:flip={{ duration: dur(FLIP) }}
          role="listitem"
          draggable={armed === b.id}
          ondragstart={(e) => dragStart(e, b)}
          ondragover={(e) => dragOver(e, i)}
          ondrop={drop}
          ondragend={dragEnd}
          class="group/block relative rounded-lg border bg-bg/60 transition-opacity
            {dragging === b.id ? 'border-accent opacity-40' : 'border-line'}"
          aria-label={info.label}
        >
          {#if dragging != null && dropAt === i}
            <div class="absolute inset-x-0 -top-1.5 h-0.5 rounded bg-accent" aria-hidden="true"></div>
          {/if}
          {#if dragging != null && dropAt === i + 1 && i === list.length - 1}
            <div class="absolute inset-x-0 -bottom-1.5 h-0.5 rounded bg-accent" aria-hidden="true"></div>
          {/if}
          <header class="flex items-center gap-1.5 px-2 pt-1.5 text-[11px] text-muted">
            <span
              role="presentation"
              onmousedown={(e) => {
                e.stopPropagation();
                armed = b.id;
              }}
              onmouseup={() => (armed = null)}
              title="Glisser pour réordonner"
              class="cursor-grab rounded px-0.5 hover:bg-raised hover:text-fg active:cursor-grabbing">⠿</span
            >
            <span aria-hidden="true">{info.icon}</span>
            <span class="font-medium tracking-wide uppercase">{info.label}</span>
            <span class="ml-auto flex gap-0.5 opacity-0 transition group-hover/block:opacity-100 group-focus-within/block:opacity-100">
              <button
                onclick={() => move(i, i - 1)}
                disabled={i === 0}
                class="rounded px-1.5 py-0.5 hover:bg-raised hover:text-fg disabled:opacity-30"
                title="Monter"
                aria-label={`Monter le bloc ${info.label}`}>↑</button
              >
              <button
                onclick={() => move(i, i + 2)}
                disabled={i === list.length - 1}
                class="rounded px-1.5 py-0.5 hover:bg-raised hover:text-fg disabled:opacity-30"
                title="Descendre"
                aria-label={`Descendre le bloc ${info.label}`}>↓</button
              >
              <button
                onclick={() => duplicateBlock(b)}
                class="rounded px-1.5 py-0.5 hover:bg-raised hover:text-fg"
                title="Dupliquer"
                aria-label={`Dupliquer le bloc ${info.label}`}>⧉</button
              >
              <button
                onclick={() => deleteBlock(b)}
                class="rounded px-1.5 py-0.5 hover:bg-raised hover:text-danger"
                title="Supprimer"
                aria-label={`Supprimer le bloc ${info.label}`}>🗑</button
              >
            </span>
          </header>
          <div class="px-2.5 pt-1 pb-2.5">
            {#if b.kind === "text"}
              <TextBlock block={as(b, "text")} />
            {:else if b.kind === "list"}
              <ListBlock block={as(b, "list")} />
            {:else if b.kind === "counter"}
              <CounterBlock block={as(b, "counter")} />
            {:else if b.kind === "image"}
              <ImageBlock block={as(b, "image")} {imagesDir} />
            {:else if b.kind === "video"}
              <VideoBlock block={as(b, "video")} />
            {:else if b.kind === "timer"}
              <TimerBlock block={as(b, "timer")} />
            {/if}
          </div>
        </section>
      {/each}
    </div>
  {/if}

  <!-- Ajout d'un bloc : le menu prend place dans la carte (qui grandit dans le tableau) -->
  <div
    bind:this={menuWrap}
    class="relative"
    onfocusout={(e) => {
      if (!e.currentTarget.contains(e.relatedTarget as Node | null)) menuOpen = false;
    }}
  >
    <button
      onclick={() => (menuOpen = !menuOpen)}
      aria-expanded={menuOpen}
      aria-haspopup="menu"
      class="flex w-full items-center justify-center gap-1.5 rounded-lg border border-dashed border-line py-1.5 text-sm text-muted transition hover:border-accent hover:text-fg"
    >
      <span class="text-base leading-none">+</span> Ajouter un bloc
    </button>
    {#if menuOpen}
      <div
        role="menu"
        tabindex="-1"
        in:pop={{ y: -4, start: 0.98 }}
        onkeydown={(e) => e.key === "Escape" && (menuOpen = false)}
        class="mt-1 grid grid-cols-2 gap-1 rounded-lg border border-line bg-raised p-1.5"
      >
        {#each blockKinds as k, i (k.kind)}
          <button
            role="menuitem"
            onclick={() => add(k.kind)}
            {@attach (el) => {
              if (i === 0) el.focus();
            }}
            class="flex items-start gap-2 rounded-md p-2 text-left transition hover:bg-panel focus-visible:bg-panel focus-visible:outline-none"
          >
            <span class="grid h-6 w-6 shrink-0 place-items-center rounded bg-bg text-sm" aria-hidden="true">{k.icon}</span>
            <span class="min-w-0">
              <span class="block text-sm font-medium">{k.label}</span>
              <span class="block text-[11px] leading-tight text-muted">{k.hint}</span>
            </span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>
