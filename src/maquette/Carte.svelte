<script lang="ts">
  // Maquette : un succès. Sans bloc → ligne compacte ; avec des blocs → carte complète.
  // En mode déplacement, toute la carte devient une cible cliquable.
  import BlocVue from "./BlocVue.svelte";
  import { newBloc, TAGS, type BlocKind, type Succes } from "./donnees";
  import { rarityText } from "../lib/filters";
  import { tagStyle } from "../lib/tags.svelte";

  let {
    s,
    moving,
    swap,
    canMove,
    flash,
    onstartmove,
    onpick,
  }: {
    s: Succes;
    /** id du succès en cours de déplacement, ou null. */
    moving: number | null;
    swap: boolean;
    canMove: boolean;
    flash: boolean;
    onstartmove: () => void;
    onpick: () => void;
  } = $props();

  const KINDS: { kind: BlocKind; icon: string; label: string }[] = [
    { kind: "video", icon: "▶", label: "Vidéo YouTube" },
    { kind: "image", icon: "▣", label: "Image" },
    { kind: "text", icon: "¶", label: "Texte" },
    { kind: "list", icon: "☑", label: "Liste" },
    { kind: "counter", icon: "#", label: "Compteur" },
    { kind: "timer", icon: "⏱", label: "Chronomètre" },
  ];

  const full = $derived(s.blocks.length > 0);
  const isMoving = $derived(moving === s.id);
  const isTarget = $derived(moving != null && !isMoving);
  const hue = $derived((s.id * 47) % 360);

  function add(kind: BlocKind) {
    s.blocks.push(newBloc(kind));
  }
  function moveBloc(i: number, d: -1 | 1) {
    const [x] = s.blocks.splice(i, 1);
    s.blocks.splice(i + d, 0, x);
  }
</script>

{#snippet icon(size: string)}
  <div
    class="{size} grid shrink-0 place-items-center rounded-[3px] font-bold text-white/90 {s.unlocked ? '' : 'opacity-60 grayscale'}"
    style="background: linear-gradient(135deg, hsl({hue} 55% 42%), hsl({(hue + 40) % 360} 50% 22%))"
    aria-hidden="true"
  >
    {s.name[0]}
  </div>
{/snippet}

{#snippet check()}
  <button
    onclick={() => (s.unlocked = !s.unlocked)}
    title={s.unlocked ? "Marquer comme non obtenu" : "Marquer comme obtenu"}
    class="grid h-5 w-5 shrink-0 place-items-center rounded-[4px] border-2 transition
      {s.unlocked ? 'border-ok bg-ok text-bg' : 'border-line hover:border-muted'}"
  >
    {#if s.unlocked}
      <svg viewBox="0 0 16 16" class="h-3 w-3" aria-hidden="true"><path d="M3 8.5l3.2 3L13 4.5" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" /></svg>
    {/if}
  </button>
{/snippet}

{#snippet meta()}
  {#if s.unlocked}<span class="font-medium text-ok">Obtenu</span><span class="text-muted">·</span>{/if}
  <span class="font-semibold whitespace-nowrap {rarityText(s.rarity)}">{s.rarity.toFixed(1)} %</span>
  {#each s.tags as t (t)}
    <span class="rounded-full border px-1.5 py-px text-[10px] leading-4 font-medium whitespace-nowrap" style={tagStyle(TAGS[t])}>{t}</span>
  {/each}
{/snippet}

{#snippet tools()}
  <!-- Ajout de bloc + déplacer : au survol seulement sur une ligne compacte -->
  <span class="flex items-center gap-0.5">
    {#each KINDS as k (k.kind)}
      <button
        onclick={() => add(k.kind)}
        title="Ajouter : {k.label}"
        class="grid h-6 w-6 place-items-center rounded text-xs text-muted hover:bg-raised hover:text-fg">{k.icon}</button
      >
    {/each}
  </span>
{/snippet}

{#snippet moveButton()}
  <button
    onclick={onstartmove}
    disabled={!canMove}
    title={canMove ? "Déplacer ce succès" : "Pour déplacer, choisir le tri « Mon ordre »"}
    class="grid h-6 shrink-0 place-items-center rounded px-1.5 text-xs text-muted hover:bg-raised hover:text-fg disabled:opacity-30"
  >
    ⇅
  </button>
{/snippet}

<div
  class="relative {flash ? 'flash' : ''}"
  role="presentation"
>
  {#if full}
    <article
      class="card group rounded-[4px] border border-l-[3px] bg-panel {isMoving ? 'ring-2 ring-accent' : ''}"
      style:border-left-color={s.unlocked ? "var(--color-ok)" : undefined}
    >
      <header class="head flex items-start gap-2 rounded-t-[3px] p-2">
        <div class="pt-2">{@render check()}</div>
        {@render icon("h-9 w-9 text-base")}
        <div class="min-w-0 flex-1">
          <div class="flex items-start gap-1">
            <span class="min-w-0 leading-snug font-medium">{s.name}</span>
            <span class="mt-0.5 shrink-0 text-xs text-muted" title="Aide">🔍</span>
          </div>
          <div class="mt-0.5 flex flex-wrap items-center gap-x-1.5 gap-y-1 text-xs">{@render meta()}</div>
        </div>
        <div class="opacity-0 transition-opacity group-hover:opacity-100">{@render moveButton()}</div>
      </header>
      <div class="space-y-2 border-t border-line px-3 pt-2 pb-3">
        <p class="text-sm text-muted">{s.description}</p>
        <div class="flex items-center text-muted/70 group-hover:text-muted"><span class="mr-0.5 text-sm">+</span>{@render tools()}</div>
        {#each s.blocks as bl, i (bl.id)}
          <BlocVue
            bloc={bl}
            first={i === 0}
            last={i === s.blocks.length - 1}
            onremove={() => s.blocks.splice(i, 1)}
            onmove={(d) => moveBloc(i, d)}
          />
        {/each}
      </div>
    </article>
  {:else}
    <!-- Ligne compacte : un succès sans guide prend le moins de place possible -->
    <article
      class="card group flex items-center gap-2 rounded-[4px] border border-l-[3px] bg-panel px-2 py-1.5 {isMoving ? 'ring-2 ring-accent' : ''}"
      style:border-left-color={s.unlocked ? "var(--color-ok)" : undefined}
    >
      {@render check()}
      {@render icon("h-7 w-7 text-sm")}
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-1.5">
          <span class="truncate text-sm font-medium">{s.name}</span>
          <span class="flex shrink-0 items-center gap-1.5 text-xs">{@render meta()}</span>
        </div>
        <div class="truncate text-xs text-muted">{s.description}</div>
      </div>
      <div class="hidden shrink-0 items-center group-hover:flex">{@render tools()}{@render moveButton()}</div>
    </article>
  {/if}

  {#if isTarget}
    <!-- Cible du déplacement : toute la carte -->
    <button
      onclick={onpick}
      class="target absolute inset-0 z-10 flex items-start justify-center rounded-[4px] border-2 border-dashed border-transparent
        text-xs font-semibold text-transparent transition hover:border-accent hover:bg-accent/15 hover:text-fg"
    >
      <span class="mt-1 rounded bg-accent px-2 py-0.5 text-on-accent">{swap ? "⇄ Échanger avec celui-ci" : "↑ Placer ici, avant celui-ci"}</span>
    </button>
  {/if}
</div>

<style>
  .card {
    border-color: color-mix(in srgb, var(--color-line) 60%, var(--color-muted));
    box-shadow:
      0 1px 2px rgb(0 0 0 / 0.2),
      0 6px 16px -8px rgb(0 0 0 / 0.45);
  }
  .head {
    background: color-mix(in srgb, var(--color-raised) 70%, var(--color-panel));
  }
  .target:not(:hover) span {
    visibility: hidden;
  }
  .flash .card {
    animation: flash 1.2s ease-out;
  }
  @keyframes flash {
    0% {
      box-shadow: 0 0 0 3px var(--color-accent);
    }
    100% {
      box-shadow: 0 0 0 0 transparent;
    }
  }
</style>
