<script lang="ts">
  import CopyId from "./CopyId.svelte";
  import HelpSearch from "./HelpSearch.svelte";
  import MoreMenu from "./MoreMenu.svelte";
  import { openRelock } from "./settings.svelte";
  import TagPicker from "./TagPicker.svelte";
  import { rarityBar, rarityText } from "./filters";
  import type { AchievementRow } from "./library";
  import SteamProgress from "./SteamProgress.svelte";

  let {
    a,
    spoiler,
    animate,
    selected,
    inWorkspace,
    ontoggle,
    onreveal,
    onselect,
    ondragstart,
    ondragend,
  }: {
    a: AchievementRow;
    /** Nom et description à flouter (succès caché, non obtenu, anti-spoiler actif). */
    spoiler: boolean;
    /** Joue l'animation de coche (seulement juste après un clic). */
    animate: boolean;
    selected: boolean;
    inWorkspace: boolean;
    ontoggle: (a: AchievementRow) => void;
    onreveal: (a: AchievementRow) => void;
    onselect: (a: AchievementRow, range: boolean) => void;
    ondragstart: (a: AchievementRow, e: DragEvent) => void;
    ondragend: () => void;
  } = $props();

  const icon = $derived(a.unlocked ? a.icon_url : (a.icon_gray_url ?? a.icon_url));
  const label = $derived(spoiler ? "succès caché" : a.name);

  const fmtDate = (t: number) =>
    new Date(t * 1000).toLocaleDateString("fr-FR", { day: "numeric", month: "short", year: "numeric" });
</script>

<li
  draggable={!inWorkspace && !a.removed}
  ondragstart={(e) => ondragstart(a, e)}
  {ondragend}
  class="group flex items-center gap-3 px-3 py-2.5 transition-colors
    {selected ? 'bg-accent/10' : 'hover:bg-raised/50'}
    {a.removed ? 'opacity-50' : ''}
    {!inWorkspace && !a.removed ? 'cursor-grab active:cursor-grabbing' : ''}"
>
  <!-- Sélection pour l'espace de travail -->
  {#if inWorkspace}
    <span
      class="grid h-4 w-4 shrink-0 place-items-center text-[11px] text-accent"
      title="Déjà dans l'espace de travail"
      aria-label="Déjà dans l'espace de travail">◆</span
    >
  {:else}
    <input
      type="checkbox"
      checked={selected}
      disabled={a.removed}
      onclick={(e) => onselect(a, e.shiftKey)}
      aria-label={`Sélectionner « ${label} » pour l'espace de travail`}
      title="Sélectionner pour l'espace de travail (Maj+clic : plage)"
      class="h-4 w-4 shrink-0 cursor-pointer accent-accent"
    />
  {/if}

  <!-- Coche « obtenu » -->
  <button
    onclick={() => ontoggle(a)}
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

  <!-- Icône -->
  <div
    class="h-11 w-11 shrink-0 overflow-hidden rounded-md bg-raised
      {a.unlocked ? 'shadow-[0_0_14px_-3px] shadow-accent/60' : ''}
      {animate && a.unlocked ? 'light-up' : ''}"
  >
    {#if icon}
      <img src={icon} alt="" loading="lazy" draggable="false" class="h-full w-full {a.unlocked ? '' : 'opacity-70 grayscale'}" />
    {/if}
  </div>

  <!-- Texte -->
  <div class="min-w-0 flex-1">
    {#if spoiler}
      <button
        onclick={() => onreveal(a)}
        class="block w-full rounded text-left focus-visible:outline-2 focus-visible:outline-accent"
        title="Succès caché : cliquer pour révéler"
      >
        <span class="flex items-center gap-2">
          <span class="truncate font-medium blur-[6px] select-none" aria-hidden="true">{a.name}</span>
          <span class="shrink-0 rounded bg-raised px-1.5 py-0.5 text-[10px] text-muted">caché · cliquer pour révéler</span>
        </span>
        <span class="sr-only">Succès caché, activer pour révéler</span>
        <span class="block truncate text-sm text-muted blur-[6px] select-none" aria-hidden="true">{a.description || "—"}</span>
      </button>
    {:else}
      <div class="flex items-center gap-2">
        <span class="truncate font-medium {a.unlocked ? '' : 'text-fg/85'}">{a.name}</span>
        {#if a.hidden}<span class="shrink-0 rounded bg-raised px-1.5 py-0.5 text-[10px] text-muted">caché</span>{/if}
        {#if a.dlc_appid}<span class="shrink-0 rounded bg-raised px-1.5 py-0.5 text-[10px] text-muted">DLC</span>{/if}
        {#if a.removed}<span class="shrink-0 rounded bg-raised px-1.5 py-0.5 text-[10px] text-muted">retiré de Steam</span>{/if}
      </div>
      <p class="text-sm text-muted">{a.description || "—"}</p>
      {#if a.progress && !a.unlocked}<div class="mt-1 max-w-64"><SteamProgress progress={a.progress} /></div>{/if}
    {/if}
    <div class="mt-0.5 flex flex-wrap items-center gap-1.5">
      {#if a.api_name}<CopyId id={a.api_name} />{/if}
      {#if !spoiler}<HelpSearch name={a.name} />{/if}
      <TagPicker achievementId={a.id} compact />
      <MoreMenu
        class="ml-auto opacity-0 transition group-hover:opacity-100 focus-within:opacity-100"
        items={[
          {
            label: "Reverrouiller dans Steam…",
            hint: "Prépare la commande console achievement_clear",
            onclick: () => openRelock(a),
          },
        ]}
      />
    </div>
  </div>

  <!-- Rareté et date -->
  <div class="w-24 shrink-0 text-right text-xs">
    {#if a.rarity != null}
      <div class="font-semibold {rarityText(a.rarity)}">{a.rarity.toFixed(1)} %</div>
      <div class="mt-1 ml-auto h-1 w-14 overflow-hidden rounded-full bg-line" aria-hidden="true">
        <div class="h-full rounded-full {rarityBar(a.rarity)}" style="width: {Math.max(a.rarity, 2)}%"></div>
      </div>
    {/if}
    {#if a.unlocked}
      <div class="mt-1 text-ok">
        {#if a.unlock_time}{fmtDate(a.unlock_time)}{:else if a.manual}coché à la main{:else}obtenu{/if}
      </div>
    {/if}
  </div>
</li>

