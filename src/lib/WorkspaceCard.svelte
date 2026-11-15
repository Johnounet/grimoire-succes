<script lang="ts">
  // Carte de la vue des succès : celle d'un succès (`a`), ou une carte libre (`free`) avec un titre
  // modifiable et des blocs, sans description ni étiquettes.
  // - Succès sans bloc : ligne compacte (le moins de place possible) ; les symboles d'ajout de bloc
  //   apparaissent au survol, et la ligne devient une carte complète dès le premier bloc.
  // - Succès avec des blocs, carte libre : carte complète.
  // Déplacement par clics (⇅) : géré par la vue (Workspace.svelte), qui pose une cible sur chaque carte.
  import BlockList from "./blocks/BlockList.svelte";
  import { blocksOf, createBlock, imageFrom, saveBlock, uploadImage } from "./blocks/store.svelte";
  import HelpSearch from "./HelpSearch.svelte";
  import MoreMenu from "./MoreMenu.svelte";
  import { openRelock } from "./settings.svelte";
  import TagPicker from "./TagPicker.svelte";
  import { rarityText } from "./filters";
  import type { AchievementRow, CardKey, FreeCard } from "./library";
  import SteamProgress from "./SteamProgress.svelte";

  let {
    a = null,
    free = null,
    focusTitle = false,
    pinned,
    spoiler,
    animate,
    imagesDir,
    canMove,
    moving,
    ontoggle,
    onreveal,
    onremove,
    onpin,
    onmove,
    onrename,
    moves = [],
  }: {
    a?: AchievementRow | null;
    free?: FreeCard | null;
    /** Carte libre qui vient d'être créée : son titre prend le focus. */
    focusTitle?: boolean;
    /** Affichée dans le mode compact. */
    pinned: boolean;
    spoiler: boolean;
    animate: boolean;
    imagesDir: string;
    /** Le tri « Mon ordre » est choisi : la carte peut être déplacée. */
    canMove: boolean;
    /** Cette carte est celle qu'on déplace. */
    moving: boolean;
    ontoggle: () => void;
    onreveal: () => void;
    /** Carte libre : supprimée avec ses blocs. */
    onremove: () => void;
    onpin: () => void;
    /** Commence le déplacement de cette carte. */
    onmove: () => void;
    /** Nouveau titre d'une carte libre. */
    onrename?: (title: string) => void;
    /** Entrées « Déplacer vers… » du menu (sections). */
    moves?: { label: string; onclick: () => void }[];
  } = $props();

  const key: CardKey = $derived(a ? a.id : -(free?.id ?? 0));
  const icon = $derived(a ? (a.unlocked ? a.icon_url : (a.icon_gray_url ?? a.icon_url)) : null);
  const label = $derived(a ? (spoiler ? "succès caché" : a.name) : free?.title || "Carte sans titre");
  const blockCount = $derived(blocksOf(key).length);
  /** Succès sans guide : ligne compacte. */
  const compact = $derived(!!a && blockCount === 0);
  /** Succès obtenu ou carte libre cochée : coche pleine et liseré gauche vert. */
  const checked = $derived(a ? a.unlocked : !!free?.done);
  /** Carte libre avec des blocs : la suppression demande confirmation (rien ne les garde ailleurs). */
  let confirming = $state(false);

  function rename(input: HTMLInputElement) {
    const title = input.value.trim();
    if (free && title !== free.title) onrename?.(title);
  }

  function askRemove() {
    if (blockCount > 0) confirming = true;
    else onremove();
  }

  /** Ctrl+V d'une capture n'importe où sur la carte (hors champ de texte) : nouveau bloc image. */
  async function paste(e: ClipboardEvent) {
    const target = e.target as HTMLElement;
    if (target.closest("input, textarea, [contenteditable]")) return;
    const file = imageFrom(e.clipboardData);
    if (!file) return;
    e.preventDefault();
    const b = await createBlock(key, "image");
    if (!b || b.kind !== "image") return;
    try {
      (b.data as { file: string | null }).file = await uploadImage(file);
      saveBlock(b);
    } catch {
      /* le bloc reste vide : l'utilisateur peut réessayer */
    }
  }

  const menu = $derived([
    {
      label: pinned ? "Retirer du mode compact" : "Épingler au mode compact",
      hint: "Cartes suivies dans la petite fenêtre au premier plan",
      onclick: onpin,
    },
    ...(a
      ? [
          {
            label: "Reverrouiller dans Steam…",
            hint: "Prépare la commande console achievement_clear",
            onclick: () => openRelock(a),
          },
          ...(a.api_name
            ? [
                {
                  label: "Copier l'ID technique",
                  hint: a.api_name,
                  onclick: () => navigator.clipboard.writeText(a.api_name ?? "").catch(() => {}),
                },
              ]
            : []),
        ]
      : []),
    ...moves,
  ]);
</script>

{#snippet checkbox(size: string)}
  <button
    onclick={ontoggle}
    disabled={!!a?.removed}
    aria-pressed={checked}
    aria-label={a
      ? checked ? `Marquer « ${label} » comme non obtenu` : `Marquer « ${label} » comme obtenu`
      : checked ? `Marquer « ${label} » comme à faire` : `Marquer « ${label} » comme faite`}
    title={a
      ? checked ? "Marquer comme non obtenu" : "Marquer comme obtenu"
      : checked ? "Marquer comme à faire" : "Marquer comme faite"}
    class="grid {size} shrink-0 place-items-center rounded-[4px] border-2 transition disabled:opacity-40
      focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent
      {checked ? 'border-ok bg-ok text-bg' : 'border-line hover:border-muted'}"
  >
    {#if checked}
      <svg viewBox="0 0 16 16" class="h-3.5 w-3.5" aria-hidden="true">
        <path d="M3 8.5l3.2 3L13 4.5" fill="none" stroke="currentColor" stroke-width="2.4"
          stroke-linecap="round" stroke-linejoin="round" class={animate ? "check-draw" : ""} />
      </svg>
    {/if}
  </button>
{/snippet}

{#snippet achIcon(a: AchievementRow, size: string)}
  <div
    class="{size} shrink-0 overflow-hidden rounded-[3px] bg-raised
      {a.unlocked ? 'shadow-[0_0_14px_-3px] shadow-accent/60' : ''}
      {animate && a.unlocked ? 'light-up' : ''}"
  >
    {#if icon}
      <img src={icon} alt="" draggable="false" class="h-full w-full {a.unlocked ? '' : 'opacity-70 grayscale'}" />
    {/if}
  </div>
{/snippet}

<!-- Rareté, étiquettes, « Obtenu », « retiré de Steam » -->
{#snippet meta(a: AchievementRow)}
  {#if a.unlocked}<span class="font-medium text-ok">Obtenu</span>{/if}
  {#if a.unlocked && a.rarity != null}<span class="text-muted">·</span>{/if}
  {#if a.rarity != null}<span class="font-semibold whitespace-nowrap {rarityText(a.rarity)}">{a.rarity.toFixed(1)} %</span>{/if}
  {#if a.removed}<span class="rounded bg-raised px-1.5 py-px text-[10px] whitespace-nowrap text-muted">retiré de Steam</span>{/if}
  <span class="cursor-auto"><TagPicker achievementId={a.id} compact /></span>
{/snippet}

<!-- Actions rares : ⇅, épingle, ⋯, ✕ (carte libre) -->
{#snippet actions()}
  <button
    onclick={onmove}
    disabled={!canMove}
    aria-label={`Déplacer « ${label} »`}
    title={canMove ? "Déplacer : cliquez ensuite sur le succès devant lequel le placer" : "Pour déplacer, choisissez le tri « Mon ordre »"}
    class="grid h-7 w-7 shrink-0 place-items-center rounded text-muted transition hover:bg-raised hover:text-fg disabled:opacity-30"
  >
    ⇅
  </button>
  <MoreMenu class="shrink-0" items={menu} />
  {#if free}
    <button
      onclick={askRemove}
      aria-label="Supprimer la carte"
      title="Supprimer la carte et ses blocs"
      class="grid h-7 w-7 shrink-0 place-items-center rounded text-muted transition hover:bg-raised hover:text-danger"
    >
      ✕
    </button>
  {/if}
{/snippet}

{#snippet pin()}
  {#if pinned}
    <button
      onclick={onpin}
      aria-label="Retirer du mode compact"
      title="Épinglée au mode compact (cliquer pour retirer)"
      class="grid h-6 w-6 shrink-0 place-items-center rounded text-accent transition hover:bg-raised"
    >
      <svg viewBox="0 0 16 16" class="h-3.5 w-3.5" aria-hidden="true">
        <path d="M6 1.5h4l-.6 4.2L12 8.5H4l2.6-2.8z M8 8.5V14.5" fill="currentColor" stroke="currentColor"
          stroke-width="1.3" stroke-linejoin="round" />
      </svg>
    </button>
  {/if}
{/snippet}

{#if compact && a}
  <!-- Ligne compacte : succès sans guide -->
  <article
    class="ws-card group relative flex items-center gap-2 rounded-[4px] border border-l-[3px] bg-panel py-1.5 pr-2 pl-2
      {a.removed ? 'opacity-60' : ''} {moving ? 'ring-2 ring-accent' : ''}"
    style:border-left-color={checked ? "var(--color-ok)" : undefined}
    aria-label={label}
    onpaste={paste}
  >
    {@render checkbox("h-5 w-5")}
    {@render achIcon(a, "h-8 w-8")}
    <div class="min-w-0 flex-1">
      {#if spoiler}
        <button onclick={onreveal} class="block max-w-full text-left" title="Succès caché : cliquer pour révéler">
          <span class="block truncate text-sm font-medium blur-[6px] select-none" aria-hidden="true">{a.name}</span>
          <span class="block truncate text-xs text-muted blur-[5px] select-none" aria-hidden="true">{a.description || "—"}</span>
          <span class="sr-only">Succès caché, activer pour révéler</span>
        </button>
      {:else}
        <div class="flex min-w-0 items-center gap-1.5">
          <span class="truncate text-sm font-medium {a.unlocked ? '' : 'text-fg/90'}">{a.name}</span>
          <HelpSearch name={a.name} iconOnly />
          <span class="flex shrink-0 items-center gap-1.5 text-xs">{@render meta(a)}</span>
        </div>
        <div class="truncate text-xs text-muted" title={a.description}>{a.description || "—"}</div>
        {#if a.progress && !a.unlocked}<div class="mt-0.5 max-w-60"><SteamProgress progress={a.progress} /></div>{/if}
      {/if}
    </div>
    {@render pin()}
    <!-- Au survol : symboles d'ajout de bloc et actions, par-dessus la fin de la ligne -->
    <div
      class="absolute inset-y-0 right-0 hidden items-center gap-1 rounded-r-[4px] bg-panel pr-1.5 pl-3 shadow-[-12px_0_12px_-4px] shadow-panel
        group-focus-within:flex group-hover:flex has-[[aria-expanded=true]]:flex"
    >
      {#if !a.removed}<BlockList cardKey={key} {imagesDir} />{/if}
      {@render actions()}
    </div>
  </article>
{:else}
  <article
    class="ws-card group rounded-[4px] border border-l-[3px] bg-panel transition-colors {moving ? 'ring-2 ring-accent' : ''}"
    style:border-left-color={checked ? "var(--color-ok)" : undefined}
    aria-label={label}
    onpaste={paste}
  >
    <header class="relative flex shrink-0 items-center gap-2 rounded-t-[3px] p-2">
      {@render checkbox("h-6 w-6")}
      {#if a}{@render achIcon(a, "h-9 w-9")}{/if}

      {#if free}
        <!-- Titre modifiable -->
        <input
          value={free.title}
          placeholder="Carte sans titre"
          aria-label="Titre de la carte"
          maxlength="200"
          onchange={(e) => rename(e.currentTarget)}
          onkeydown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
            if (e.key === "Escape") {
              e.currentTarget.value = free.title;
              e.currentTarget.blur();
            }
          }}
          {@attach (el) => {
            if (focusTitle) el.focus();
          }}
          class="min-w-0 flex-1 rounded-md border border-transparent bg-transparent px-1.5 py-1 font-medium outline-none
            placeholder:text-muted hover:border-line focus:border-accent focus:bg-bg"
        />
      {:else if a}
        <div class="min-w-0 flex-1">
          {#if spoiler}
            <button onclick={onreveal} class="block max-w-full text-left" title="Succès caché : cliquer pour révéler">
              <span class="block truncate font-medium blur-[6px] select-none" aria-hidden="true">{a.name}</span>
              <span class="sr-only">Succès caché, activer pour révéler</span>
            </button>
          {:else}
            <div class="flex items-start gap-1">
              <div class="min-w-0 leading-snug font-medium break-words {a.unlocked ? '' : 'text-fg/90'}">{a.name}</div>
              <HelpSearch name={a.name} iconOnly />
            </div>
          {/if}
          <div class="flex flex-wrap items-center gap-x-1.5 gap-y-1 text-xs">{@render meta(a)}</div>
        </div>
      {/if}

      {@render pin()}
      <!-- Actions rares : visibles au survol (ou au clavier), pour laisser toute la place au nom -->
      <div
        class="absolute top-1.5 right-1.5 flex items-center gap-0.5 rounded border border-line bg-panel opacity-0 transition-opacity
          group-hover:opacity-100 focus-within:opacity-100 has-[[aria-expanded=true]]:opacity-100"
      >
        {@render actions()}
      </div>
    </header>

    {#if confirming}
      <div class="mx-2.5 mb-2.5 flex flex-wrap items-center gap-2 rounded-lg border border-danger/40 bg-danger/10 p-2 text-sm">
        <span class="min-w-0 flex-1">Supprimer cette carte et {blockCount > 1 ? `ses ${blockCount} blocs` : "son bloc"} ?</span>
        <button onclick={() => (confirming = false)} class="rounded-md px-2 py-0.5 hover:bg-raised">Annuler</button>
        <button onclick={onremove} class="rounded-md bg-danger px-2 py-0.5 font-medium text-white">Supprimer</button>
      </div>
    {/if}

    <div class="border-t border-line px-3 pt-2 pb-3">
      <div class="space-y-2">
        {#if a && spoiler}
          <button onclick={onreveal} class="block text-left text-sm text-muted">
            <span class="blur-[6px] select-none" aria-hidden="true">{a.description || "—"}</span>
            <span class="ml-1 rounded bg-raised px-1.5 py-0.5 text-[10px]">caché · cliquer pour révéler</span>
          </button>
        {:else if a}
          <p class="text-sm text-muted">{a.description || "—"}</p>
          {#if a.progress && !a.unlocked}<SteamProgress progress={a.progress} />{/if}
        {/if}

        <BlockList cardKey={key} {imagesDir} />
      </div>
    </div>
  </article>
{/if}

<style>
  /* Cartes bien détachées les unes des autres : bordure plus marquée que celle des blocs,
     ombre portée et en-tête teinté (demande de l'utilisateur, 2026-11-15) */
  .ws-card {
    border-color: color-mix(in srgb, var(--color-line) 60%, var(--color-muted));
    box-shadow:
      0 1px 2px rgb(0 0 0 / 0.2),
      0 6px 16px -8px rgb(0 0 0 / 0.45);
  }
  .ws-card > header {
    background: color-mix(in srgb, var(--color-raised) 70%, var(--color-panel));
  }
</style>
