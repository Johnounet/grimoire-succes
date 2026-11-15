<script lang="ts">
  // Succès rattaché à un autre (demande de l'utilisateur, 2026-11-15) : il n'a plus sa propre carte et
  // s'affiche en ligne dans la carte de son parent. Sa case reste cochable (Steam le débloque à part),
  // et ses blocs de guide restent visibles (repliables par le chevron).
  import BlockList from "./blocks/BlockList.svelte";
  import { blocksOf } from "./blocks/store.svelte";
  import { rarityText } from "./filters";
  import HelpSearch from "./HelpSearch.svelte";
  import type { AchievementRow } from "./library";
  import MoreMenu from "./MoreMenu.svelte";
  import { openRelock } from "./settings.svelte";
  import SteamProgress from "./SteamProgress.svelte";

  let {
    a,
    spoiler,
    animate,
    folded,
    imagesDir,
    ontoggle,
    onreveal,
    ondetach,
    onfold,
  }: {
    a: AchievementRow;
    spoiler: boolean;
    animate: boolean;
    /** Blocs du succès repliés. */
    folded: boolean;
    imagesDir: string;
    ontoggle: () => void;
    onreveal: () => void;
    /** Rend au succès sa propre carte. */
    ondetach: () => void;
    onfold: () => void;
  } = $props();

  const label = $derived(spoiler ? "succès caché" : a.name);
  const icon = $derived(a.unlocked ? a.icon_url : (a.icon_gray_url ?? a.icon_url));
  const blockCount = $derived(blocksOf(a.id).length);

  const menu = $derived([
    { label: "Détacher", hint: "Le succès retrouve sa propre carte, juste après celle-ci", onclick: ondetach },
    { label: "Reverrouiller dans Steam…", hint: "Prépare la commande console achievement_clear", onclick: () => openRelock(a) },
    ...(a.api_name
      ? [
          {
            label: "Copier l'ID technique",
            hint: a.api_name,
            onclick: () => navigator.clipboard.writeText(a.api_name ?? "").catch(() => {}),
          },
        ]
      : []),
  ]);
</script>

<div
  class="nested group/n rounded-[4px] border border-l-[3px] border-line bg-bg/50 {a.removed ? 'opacity-60' : ''}"
  style:border-left-color={a.unlocked ? "var(--color-ok)" : undefined}
  role="group"
  aria-label={`Succès rattaché : ${label}`}
>
  <div class="flex items-center gap-2 py-1 pr-1 pl-1.5">
    <button
      onclick={ontoggle}
      disabled={a.removed}
      aria-pressed={a.unlocked}
      aria-label={a.unlocked ? `Marquer « ${label} » comme non obtenu` : `Marquer « ${label} » comme obtenu`}
      title={a.unlocked ? "Marquer comme non obtenu" : "Marquer comme obtenu"}
      class="grid h-5 w-5 shrink-0 place-items-center rounded-[4px] border-2 transition disabled:opacity-40
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
    <div class="h-7 w-7 shrink-0 overflow-hidden rounded-[3px] bg-raised {animate && a.unlocked ? 'light-up' : ''}">
      {#if icon}<img src={icon} alt="" draggable="false" class="h-full w-full {a.unlocked ? '' : 'opacity-70 grayscale'}" />{/if}
    </div>
    <div class="min-w-0 flex-1">
      {#if spoiler}
        <button onclick={onreveal} class="block max-w-full text-left" title="Succès caché : cliquer pour révéler">
          <span class="block truncate text-sm font-medium blur-[6px] select-none" aria-hidden="true">{a.name}</span>
          <span class="sr-only">Succès caché, activer pour révéler</span>
        </button>
      {:else}
        <div class="flex min-w-0 items-center gap-1.5">
          <span class="truncate text-sm font-medium {a.unlocked ? '' : 'text-fg/90'}">{a.name}</span>
          <HelpSearch name={a.name} iconOnly />
          {#if a.rarity != null}
            <span class="shrink-0 text-xs font-semibold whitespace-nowrap {rarityText(a.rarity)}">{a.rarity.toFixed(1)} %</span>
          {/if}
          {#if a.removed}<span class="shrink-0 rounded bg-raised px-1.5 py-px text-[10px] text-muted">retiré de Steam</span>{/if}
        </div>
        <div class="truncate text-xs text-muted" title={a.description}>{a.description || "—"}</div>
        {#if a.progress && !a.unlocked}<div class="mt-0.5 max-w-60"><SteamProgress progress={a.progress} /></div>{/if}
      {/if}
    </div>
    {#if blockCount > 0}
      <button
        onclick={onfold}
        aria-expanded={!folded}
        aria-label={folded ? "Afficher son guide" : "Replier son guide"}
        title={folded ? `Afficher son guide (${blockCount} bloc${blockCount > 1 ? "s" : ""})` : "Replier son guide"}
        class="flex h-7 shrink-0 items-center gap-1 rounded px-1.5 text-xs text-muted transition hover:bg-raised hover:text-fg"
      >
        {#if folded}<span class="tabular-nums">{blockCount}</span>{/if}
        <span class="transition-transform {folded ? '-rotate-90' : ''}">▾</span>
      </button>
    {/if}
    <MoreMenu
      class="shrink-0 opacity-0 transition-opacity group-hover/n:opacity-100 focus-within:opacity-100 has-[[aria-expanded=true]]:opacity-100"
      label={`Actions de « ${label} »`}
      items={menu}
    />
  </div>
  {#if blockCount > 0 && !folded}
    <div class="border-t border-line px-2 pt-1.5 pb-2"><BlockList cardKey={a.id} {imagesDir} /></div>
  {/if}
</div>
