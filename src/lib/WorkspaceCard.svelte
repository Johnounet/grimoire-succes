<script lang="ts">
  // Carte de l'espace de travail : celle d'un succès (`a`), ou une carte libre (`free`) avec un titre
  // modifiable et des blocs, sans coche, description ni étiquettes.
  // - Vue liste : repliable, déplacée par sa poignée (svelte-dnd-action).
  // - Vue tableau : remplit sa case de la grille (gridstack), déplacée par la poignée ou le titre
  //   (classe `card-drag`), contenu qui défile si la carte est petite.
  import { dragHandle } from "svelte-dnd-action";
  import BlockList from "./blocks/BlockList.svelte";
  import { blocksOf, createBlock, imageFrom, saveBlock, uploadImage } from "./blocks/store.svelte";
  import CopyId from "./CopyId.svelte";
  import MoreMenu from "./MoreMenu.svelte";
  import { openRelock } from "./settings.svelte";
  import TagPicker from "./TagPicker.svelte";
  import { rarityText } from "./filters";
  import { columns, type AchievementRow, type CardKey, type Column, type FreeCard } from "./library";
  import SteamProgress from "./SteamProgress.svelte";

  let {
    a = null,
    free = null,
    focusTitle = false,
    column,
    collapsed,
    pinned,
    spoiler,
    animate,
    board,
    imagesDir,
    ontoggle,
    onreveal,
    oncollapse,
    onremove,
    oncolumn,
    onpin,
    onrename,
  }: {
    a?: AchievementRow | null;
    free?: FreeCard | null;
    /** Carte libre qui vient d'être créée : son titre prend le focus. */
    focusTitle?: boolean;
    column: Column;
    collapsed: boolean;
    /** Affichée dans le mode compact. */
    pinned: boolean;
    spoiler: boolean;
    animate: boolean;
    /** Carte affichée dans la vue Tableau. */
    board: boolean;
    imagesDir: string;
    ontoggle: () => void;
    onreveal: () => void;
    oncollapse: () => void;
    onremove: () => void;
    oncolumn: (c: Column) => void;
    onpin: () => void;
    /** Nouveau titre d'une carte libre. */
    onrename?: (title: string) => void;
  } = $props();

  const key: CardKey = $derived(a ? a.id : -(free?.id ?? 0));
  const icon = $derived(a ? (a.unlocked ? a.icon_url : (a.icon_gray_url ?? a.icon_url)) : null);
  const label = $derived(a ? (spoiler ? "succès caché" : a.name) : free?.title || "Carte sans titre");
  const blockCount = $derived(blocksOf(key).length);
  /** Carte libre avec des blocs : la suppression demande confirmation (rien ne les garde ailleurs). */
  let confirming = $state(false);

  function rename(input: HTMLInputElement) {
    const title = input.value.trim();
    if (free && title !== free.title) onrename?.(title);
  }

  function askRemove() {
    if (free && blockCount > 0) confirming = true;
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
</script>

<article
  class="rounded-xl border bg-panel transition-colors
    {board ? 'flex h-full flex-col overflow-hidden' : ''}
    {a?.unlocked ? 'border-ok/30' : 'border-line'}"
  aria-label={label}
  onpaste={paste}
>
  <header class="flex shrink-0 items-center gap-2 p-2.5">
    {#if board}
      <span
        class="card-drag grid h-8 w-5 shrink-0 cursor-grab place-items-center rounded text-muted hover:bg-raised hover:text-fg active:cursor-grabbing"
        title="Glisser pour déplacer · coin inférieur droit pour redimensionner"
        aria-hidden="true">⠿</span
      >
    {:else}
      <span
        use:dragHandle
        aria-label={`Déplacer la carte « ${label} »`}
        title="Glisser pour déplacer (ou Entrée puis flèches)"
        class="grid h-8 w-5 shrink-0 cursor-grab place-items-center rounded text-muted hover:bg-raised hover:text-fg active:cursor-grabbing"
        >⠿</span
      >
    {/if}

    {#if a}
    <button
      onclick={ontoggle}
      aria-pressed={a.unlocked}
      aria-label={a.unlocked ? `Marquer « ${label} » comme non obtenu` : `Marquer « ${label} » comme obtenu`}
      title={a.unlocked ? "Marquer comme non obtenu" : "Marquer comme obtenu"}
      class="grid h-6 w-6 shrink-0 place-items-center rounded-full border-2 transition
        focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent
        {a.unlocked ? 'border-ok bg-ok text-bg' : 'border-line hover:border-muted'}"
    >
      {#if a.unlocked}
        <svg viewBox="0 0 16 16" class="h-3.5 w-3.5" aria-hidden="true">
          <path d="M3 8.5l3.2 3L13 4.5" fill="none" stroke="currentColor" stroke-width="2.4"
            stroke-linecap="round" stroke-linejoin="round" class={animate ? "check-draw" : ""} />
        </svg>
      {/if}
    </button>

    <div
      class="h-10 w-10 shrink-0 overflow-hidden rounded-md bg-raised
        {a.unlocked ? 'shadow-[0_0_14px_-3px] shadow-accent/60' : ''}
        {animate && a.unlocked ? 'light-up' : ''}"
    >
      {#if icon}
        <img src={icon} alt="" draggable="false" class="h-full w-full {a.unlocked ? '' : 'opacity-70 grayscale'}" />
      {/if}
    </div>
    {/if}

    {#if free}
      <!-- Titre modifiable : la carte se déplace par sa poignée ⠿ -->
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
    <div class="min-w-0 flex-1 {board ? 'card-drag cursor-grab active:cursor-grabbing' : ''}">
      {#if spoiler}
        <button onclick={onreveal} class="block max-w-full text-left" title="Succès caché : cliquer pour révéler">
          <span class="block truncate font-medium blur-[6px] select-none" aria-hidden="true">{a.name}</span>
          <span class="sr-only">Succès caché, activer pour révéler</span>
        </button>
      {:else}
        <div class="font-medium {board ? 'line-clamp-2 leading-snug break-words' : 'truncate'} {a.unlocked ? '' : 'text-fg/90'}">
          {a.name}
        </div>
      {/if}
      <div class="flex items-center gap-2 text-xs">
        {#if a.rarity != null}<span class="font-semibold {rarityText(a.rarity)}">{a.rarity.toFixed(1)} %</span>{/if}
        {#if collapsed && !board && blockCount > 0}<span class="text-muted">· {blockCount} bloc{blockCount > 1 ? "s" : ""}</span>{/if}
      </div>
    </div>
    {/if}

    {#if pinned}
      <!-- Visible seulement une fois épinglée, pour ne pas encombrer les petites cartes -->
      <button
        onclick={onpin}
        aria-label="Retirer du mode compact"
        title="Épinglée au mode compact (cliquer pour retirer)"
        class="grid h-7 w-7 shrink-0 place-items-center rounded text-accent transition hover:bg-raised"
      >
        <svg viewBox="0 0 16 16" class="h-4 w-4" aria-hidden="true">
          <path d="M6 1.5h4l-.6 4.2L12 8.5H4l2.6-2.8z M8 8.5V14.5" fill="currentColor" stroke="currentColor"
            stroke-width="1.3" stroke-linejoin="round" />
        </svg>
      </button>
    {/if}
    <MoreMenu
      class="shrink-0"
      items={[
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
            ]
          : []),
      ]}
    />
    {#if !board}
      <button
        onclick={oncollapse}
        aria-expanded={!collapsed}
        aria-label={collapsed ? "Déplier la carte" : "Replier la carte"}
        title={collapsed ? "Déplier" : "Replier"}
        class="grid h-7 w-7 shrink-0 place-items-center rounded text-muted transition hover:bg-raised hover:text-fg"
      >
        <span class="transition-transform {collapsed ? '-rotate-90' : ''}">▾</span>
      </button>
    {/if}
    <button
      onclick={askRemove}
      aria-label={free ? "Supprimer la carte" : "Retirer de l'espace de travail"}
      title={free ? "Supprimer la carte et ses blocs" : "Retirer de l'espace de travail (ses blocs sont conservés)"}
      class="grid h-7 w-7 shrink-0 place-items-center rounded text-muted transition hover:bg-raised hover:text-danger"
    >
      ✕
    </button>
  </header>

  {#if confirming}
    <div class="mx-2.5 mb-2.5 flex flex-wrap items-center gap-2 rounded-lg border border-danger/40 bg-danger/10 p-2 text-sm">
      <span class="min-w-0 flex-1">Supprimer cette carte et {blockCount > 1 ? `ses ${blockCount} blocs` : "son bloc"} ?</span>
      <button onclick={() => (confirming = false)} class="rounded-md px-2 py-0.5 hover:bg-raised">Annuler</button>
      <button onclick={onremove} class="rounded-md bg-danger px-2 py-0.5 font-medium text-white">Supprimer</button>
    </div>
  {/if}

  {#if board || !collapsed}
    <div class="border-t border-line px-3 pt-2 pb-3 {board ? 'min-h-0 flex-1 overflow-y-auto' : ''}">
      <!-- Contenu mesuré par le tableau : la carte grandit quand il grandit (Board.svelte, watchGrowth) -->
      <div class="space-y-2" data-card-content>
      {#if !a}
        <!-- Carte libre : pas de description -->
      {:else if spoiler}
        <button onclick={onreveal} class="block text-left text-sm text-muted">
          <span class="blur-[6px] select-none" aria-hidden="true">{a.description || "—"}</span>
          <span class="ml-1 rounded bg-raised px-1.5 py-0.5 text-[10px]">caché · cliquer pour révéler</span>
        </button>
      {:else}
        <p class="text-sm text-muted">{a.description || "—"}</p>
        {#if a.progress && !a.unlocked}<SteamProgress progress={a.progress} />{/if}
      {/if}

      <div class="flex flex-wrap items-center gap-2">
        {#if a?.api_name}<CopyId id={a.api_name} />{/if}
        {#if a}<TagPicker achievementId={a.id} />{/if}
        <select
          value={column}
          onchange={(e) => oncolumn((e.currentTarget as HTMLSelectElement).value as Column)}
          aria-label="Statut"
          title="Statut"
          class="ml-auto rounded-md border border-line bg-bg px-1.5 py-0.5 text-xs outline-none focus:border-accent
            {column === 'done' ? 'text-ok' : column === 'doing' ? 'text-accent' : 'text-muted'}"
        >
          {#each columns as c (c.id)}<option value={c.id}>{c.label}</option>{/each}
        </select>
      </div>

      <BlockList cardKey={key} {imagesDir} />
      </div>
    </div>
  {/if}
</article>
