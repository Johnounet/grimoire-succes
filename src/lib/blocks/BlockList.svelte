<script lang="ts">
  // Blocs d'une carte (succès ou carte libre) : affichage, réordonnancement, ajout, duplication, suppression.
  //
  // Réordonnancement par glisser HTML natif (poignée ⠿) et boutons ↑ ↓ au clavier.
  // Pas svelte-dnd-action ici : ses poignées partagent un état global, et une poignée de bloc
  // rendrait aussi déplaçable la carte qui le contient (zones imbriquées).
  import { dur } from "../motion.svelte";
  import { flip } from "svelte/animate";
  import CounterBlock from "./CounterBlock.svelte";
  import ImageBlock from "./ImageBlock.svelte";
  import ListBlock from "./ListBlock.svelte";
  import { blocksOf, createBlock, deleteBlock, duplicateBlock, reorderBlocks, saveBlock } from "./store.svelte";
  import TextBlock from "./TextBlock.svelte";
  import TimerBlock from "./TimerBlock.svelte";
  import type { CardKey } from "../library";
  import { plainText } from "../markdown";
  import { blockKinds, formatDuration, kindInfo, type Block, type BlockKind } from "./types";
  import VideoBlock from "./VideoBlock.svelte";

  /** `cardKey` : carte d'un succès ou carte libre. */
  let { cardKey, imagesDir }: { cardKey: CardKey; imagesDir: string } = $props();

  const FLIP = 150;
  const MIME = "application/x-grimoire-block";

  /** Délai avant l'infobulle d'un symbole d'ajout : assez long pour ne pas gêner, assez court pour apprendre. */
  const TIP_DELAY = 500;
  let tip = $state<{ label: string; hint: string; x: number; y: number } | null>(null);
  let tipTimer: ReturnType<typeof setTimeout> | undefined;

  /** Infobulle sous le symbole, en position fixe : elle échappe à la carte qui coupe son contenu. */
  function showTip(el: HTMLElement, k: { label: string; hint: string }, delay: number) {
    clearTimeout(tipTimer);
    tipTimer = setTimeout(() => {
      const r = el.getBoundingClientRect();
      const x = Math.max(110, Math.min(r.left + r.width / 2, window.innerWidth - 110));
      tip = { label: k.label, hint: k.hint, x, y: r.bottom + 6 };
    }, delay);
  }
  function hideTip() {
    clearTimeout(tipTimer);
    tip = null;
  }
  $effect(() => () => clearTimeout(tipTimer));

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
    hideTip();
    await createBlock(cardKey, kind);
  }

  function toggle(b: Block) {
    b.data.collapsed = !b.data.collapsed;
    saveBlock(b);
  }

  /** Résumé d'un bloc replié, pour savoir ce qu'il contient sans le déplier. */
  function summary(b: Block): string {
    const d = b.data as unknown as Record<string, unknown>;
    switch (b.kind) {
      case "text": {
        const line = plainText(String(d.markdown ?? "").split("\n").find((l) => l.trim()) ?? "");
        return line.replace(/^[#>\-*\s]+|[*_`]/g, "").trim();
      }
      case "image":
        return String(d.caption ?? "") || (d.file ? "" : "vide");
      case "video":
        return String(d.title ?? "") || (d.url ? "" : "vide");
      case "list": {
        const items = (d.items as { checked: boolean }[]) ?? [];
        return `${items.filter((i) => i.checked).length} / ${items.length}`;
      }
      case "counter":
        return d.goal != null ? `${d.value} / ${d.goal}` : String(d.value);
      case "timer":
        if (d.running_since != null) return "en cours";
        return d.best_ms != null ? `meilleur : ${formatDuration(d.best_ms as number)}` : "";
    }
  }

  // Raccourcis typés pour les composants de chaque bloc
  const as = <K extends BlockKind>(b: Block, _k: K) => b as Block<K>;
</script>

<div class="space-y-2">
  <!-- Ajout d'un bloc : un symbole par type, nom et usage en infobulle -->
  <div
    role="toolbar"
    aria-label="Ajouter un bloc"
    class="flex items-center gap-0.5 text-muted/70 transition-colors group-hover:text-muted"
  >
    <span class="mr-0.5 text-sm leading-none" aria-hidden="true">+</span>
    {#each blockKinds as k (k.kind)}
      <button
        onclick={() => add(k.kind)}
        onmouseenter={(e) => showTip(e.currentTarget, k, TIP_DELAY)}
        onmouseleave={hideTip}
        onfocus={(e) => showTip(e.currentTarget, k, 0)}
        onblur={hideTip}
        aria-label={`Ajouter un bloc ${k.label}`}
        class="grid h-7 w-7 place-items-center rounded-md text-sm transition hover:bg-raised hover:text-fg
          focus-visible:outline-2 focus-visible:outline-accent"
      >
        <span aria-hidden="true">{k.icon}</span>
      </button>
    {/each}
  </div>
  {#if tip}
    <div
      role="tooltip"
      class="pointer-events-none fixed z-50 max-w-[220px] -translate-x-1/2 rounded-md border border-line bg-raised px-2 py-1 text-center shadow-lg"
      style="left: {tip.x}px; top: {tip.y}px"
    >
      <div class="text-xs font-medium text-fg">{tip.label}</div>
      <div class="text-[11px] leading-tight text-muted">{tip.hint}</div>
    </div>
  {/if}

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
          <header class="flex items-center gap-1.5 px-2 pt-1.5 text-[11px] text-muted {b.data.collapsed ? 'pb-1.5' : ''}">
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
            <button
              onclick={() => toggle(b)}
              aria-expanded={!b.data.collapsed}
              aria-label={b.data.collapsed ? `Déplier le bloc ${info.label}` : `Replier le bloc ${info.label}`}
              title={b.data.collapsed ? "Déplier" : "Replier"}
              class="grid h-5 w-5 shrink-0 place-items-center rounded hover:bg-raised hover:text-fg"
            >
              <span class="transition-transform {b.data.collapsed ? '-rotate-90' : ''}">▾</span>
            </button>
            <span aria-hidden="true">{info.icon}</span>
            {#if b.kind === "counter" || b.kind === "timer"}
              <!-- Nom facultatif à la place du type : repris par le mode compact et les raccourcis -->
              <input
                bind:value={(b.data as { label: string }).label}
                oninput={() => saveBlock(b)}
                placeholder={info.label}
                aria-label={`Nom du ${info.label.toLowerCase()} (facultatif)`}
                title="Nommer ce {info.label.toLowerCase()} (facultatif)"
                maxlength="80"
                class="min-w-0 flex-1 rounded border border-transparent bg-transparent px-1 font-medium text-fg outline-none
                  placeholder:tracking-wide placeholder:text-muted placeholder:uppercase hover:border-line focus:border-accent"
              />
            {:else}
              <span class="shrink-0 font-medium tracking-wide uppercase">{info.label}</span>
            {/if}
            {#if b.data.collapsed}
              {@const s = summary(b)}
              {#if s}<span class="min-w-0 truncate normal-case">· {s}</span>{/if}
            {/if}
            <span class="ml-auto flex shrink-0 gap-0.5 opacity-0 transition group-hover/block:opacity-100 group-focus-within/block:opacity-100">
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
          {#if !b.data.collapsed}
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
          {/if}
        </section>
      {/each}
    </div>
  {/if}
</div>
