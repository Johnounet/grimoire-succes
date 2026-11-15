<script lang="ts">
  // Préambule du jeu (demande de l'utilisateur, 2026-11-15) : prérequis, choses à retenir, conseils.
  // Cadre pleine largeur tout en haut de la vue, repliable, fait des mêmes blocs que les cartes. C'est une
  // carte libre marquée `intro` (un seul par jeu), hors de l'espace de travail : ni colonne, ni filtres,
  // ni bilans, ni mode compact.
  import BlockList from "./blocks/BlockList.svelte";
  import { blocksOf } from "./blocks/store.svelte";
  import type { FreeCard } from "./library";
  import MoreMenu from "./MoreMenu.svelte";

  let {
    intro,
    imagesDir,
    focusTitle,
    onrename,
    oncollapse,
    ondelete,
  }: {
    intro: FreeCard;
    imagesDir: string;
    /** Préambule qui vient d'être créé : son titre prend le focus. */
    focusTitle: boolean;
    onrename: (title: string) => void;
    oncollapse: (collapsed: boolean) => void;
    ondelete: () => void;
  } = $props();

  const key = $derived(-intro.id);
  const blockCount = $derived(blocksOf(key).length);
  let confirming = $state(false);

  function rename(input: HTMLInputElement) {
    const title = input.value.trim();
    if (title !== intro.title) onrename(title);
  }
</script>

<section aria-label={intro.title || "Préambule"} class="intro rounded-[6px] border">
  <div class="intro-head flex items-center gap-1.5 px-1.5 py-1 {intro.collapsed && !confirming ? 'rounded-[5px]' : 'rounded-t-[5px] border-b'}">
    <button
      onclick={() => oncollapse(!intro.collapsed)}
      aria-expanded={!intro.collapsed}
      aria-label={intro.collapsed ? "Déplier le préambule" : "Replier le préambule"}
      title={intro.collapsed ? "Déplier" : "Replier"}
      class="grid h-7 w-7 shrink-0 place-items-center rounded text-muted transition hover:bg-raised hover:text-fg"
    >
      <span class="transition-transform {intro.collapsed ? '-rotate-90' : ''}">▾</span>
    </button>
    <input
      value={intro.title}
      placeholder="Préambule"
      aria-label="Titre du préambule"
      maxlength="200"
      onchange={(e) => rename(e.currentTarget)}
      onkeydown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
        if (e.key === "Escape") {
          e.currentTarget.value = intro.title;
          e.currentTarget.blur();
        }
      }}
      {@attach (el) => {
        if (focusTitle) el.focus();
      }}
      class="min-w-0 flex-1 rounded-md border border-transparent bg-transparent px-1.5 py-0.5 font-semibold outline-none
        placeholder:text-fg hover:border-line focus:border-accent focus:bg-bg focus:placeholder:text-muted"
    />
    {#if intro.collapsed && blockCount > 0}
      <span class="shrink-0 text-xs text-muted">{blockCount} bloc{blockCount > 1 ? "s" : ""}</span>
    {/if}
    <MoreMenu
      class="shrink-0"
      label="Actions du préambule"
      items={[{ label: "Supprimer le préambule…", onclick: () => (blockCount > 0 ? (confirming = true) : ondelete()) }]}
    />
  </div>
  {#if confirming}
    <div class="m-2 flex flex-wrap items-center gap-2 rounded border border-danger/40 bg-danger/10 p-3 text-sm">
      <p class="min-w-0 flex-1">
        Supprimer le préambule et {blockCount > 1 ? `ses ${blockCount} blocs` : "son bloc"} ? Cette action est définitive.
      </p>
      <button onclick={() => (confirming = false)} class="rounded-md px-2 py-0.5 hover:bg-raised">Annuler</button>
      <button onclick={ondelete} class="rounded-md bg-danger px-2 py-0.5 font-medium text-white">Supprimer</button>
    </div>
  {/if}
  {#if !intro.collapsed}
    <div class="px-3 pt-2 pb-3">
      {#if blockCount === 0}
        <p class="mb-2 text-sm text-muted">
          Prérequis, choses à ne pas rater, conseils généraux… Ajoutez un bloc avec les symboles ci-dessous.
        </p>
      {/if}
      <BlockList cardKey={key} {imagesDir} />
    </div>
  {/if}
</section>

<style>
  .intro {
    border-color: color-mix(in srgb, var(--color-accent) 55%, transparent);
    background: color-mix(in srgb, var(--color-accent) 5%, var(--color-bg));
  }
  .intro-head {
    background: color-mix(in srgb, var(--color-accent) 16%, var(--color-panel));
    border-color: color-mix(in srgb, var(--color-accent) 45%, transparent);
  }
</style>
