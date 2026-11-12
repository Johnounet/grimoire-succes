<script lang="ts">
  // Panneau de gauche : jeux importés. Repliable : il ne montre alors que les jaquettes (avec un
  // liseré de progression), pour garder le changement de jeu à portée de clic.
  import { localImage, percent, type GameSummary } from "./library";
  import ProgressRing from "./ProgressRing.svelte";
  import { setSidebarOpen, settings } from "./settings.svelte";

  let {
    games,
    imagesDir,
    selected,
    onselect,
    onadd,
    onsettings,
    onabout,
  }: {
    games: GameSummary[];
    imagesDir: string;
    selected: number | null;
    onselect: (appid: number) => void;
    onadd: () => void;
    onsettings: () => void;
    onabout: () => void;
  } = $props();

  const open = $derived(settings.sidebarOpen);
</script>

{#snippet cover(g: GameSummary, pct: number)}
  {@const src = localImage(imagesDir, g.cover_file)}
  <div class="h-[60px] w-10 shrink-0 overflow-hidden rounded-md bg-raised ring-1 {pct >= 100 ? 'ring-gold' : 'ring-line'}">
    {#if src}
      <img {src} alt="" class="h-full w-full {g.cover_kind === 'header' ? 'object-cover object-left' : 'object-cover'}" />
    {/if}
  </div>
{/snippet}

<aside class="flex shrink-0 flex-col border-r border-line bg-panel transition-[width] {open ? 'w-72' : 'w-16'}">
  <div class="flex gap-1.5 p-3 {open ? '' : 'flex-col items-center px-2'}">
    <button
      onclick={onadd}
      title="Ajouter un jeu"
      aria-label="Ajouter un jeu"
      class="flex items-center justify-center gap-2 rounded-lg bg-accent py-2 text-sm font-medium text-white transition hover:brightness-110 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent
        {open ? 'flex-1 px-3' : 'w-10'}"
    >
      <span class="text-lg leading-none">+</span>{#if open}Ajouter un jeu{/if}
    </button>
    <button
      onclick={() => setSidebarOpen(!open)}
      title={open ? "Replier la liste des jeux" : "Déplier la liste des jeux"}
      aria-label={open ? "Replier la liste des jeux" : "Déplier la liste des jeux"}
      aria-expanded={open}
      class="rounded-lg px-2 text-muted hover:bg-raised hover:text-fg {open ? '' : 'w-10 py-1'}"
    >
      {open ? "‹" : "›"}
    </button>
  </div>

  <nav class="flex-1 overflow-y-auto pb-3 {open ? 'px-2' : 'px-1.5'}" aria-label="Jeux importés">
    {#if games.length === 0 && open}
      <p class="px-3 py-6 text-center text-sm text-muted">Aucun jeu pour l'instant.</p>
    {/if}
    <ul class="space-y-1">
      {#each games as g (g.appid)}
        {@const pct = percent(g)}
        <li>
          <button
            onclick={() => onselect(g.appid)}
            aria-current={selected === g.appid ? "page" : undefined}
            title={open ? undefined : `${g.name} · ${g.unlocked} / ${g.total} · ${pct} %`}
            aria-label={open ? undefined : g.name}
            class="group flex w-full items-center gap-3 rounded-lg p-1.5 text-left transition
              hover:bg-raised focus-visible:outline-2 focus-visible:outline-accent
              {selected === g.appid ? 'bg-raised' : ''} {open ? '' : 'flex-col gap-1'}"
          >
            {@render cover(g, pct)}
            {#if open}
              <div class="min-w-0 flex-1">
                <div class="truncate text-sm font-medium">{g.name}</div>
                <div class="text-xs text-muted">{g.unlocked} / {g.total} · {pct} %</div>
              </div>
              <ProgressRing value={pct} />
            {:else}
              <!-- Progression sous la jaquette -->
              <div class="h-1 w-10 overflow-hidden rounded-full bg-line" aria-hidden="true">
                <div class="h-full {pct >= 100 ? 'bg-gold' : 'bg-accent'}" style:width="{pct}%"></div>
              </div>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  </nav>

  <div class="flex gap-1 border-t border-line p-2 {open ? '' : 'flex-col items-center'}">
    <button
      onclick={onsettings}
      title="Réglages"
      aria-label={open ? undefined : "Réglages"}
      class="flex items-center gap-2 rounded-lg py-2 text-sm text-muted transition hover:bg-raised hover:text-fg {open ? 'flex-1 px-3' : 'w-10 justify-center'}"
    >
      <span aria-hidden="true">⚙</span>{#if open}Réglages{/if}
    </button>
    <button
      onclick={onabout}
      title="À propos"
      aria-label={open ? undefined : "À propos"}
      class="flex items-center gap-2 rounded-lg py-2 text-sm text-muted transition hover:bg-raised hover:text-fg {open ? 'px-3' : 'w-10 justify-center'}"
    >
      <span aria-hidden="true">ⓘ</span>{#if open}À propos{/if}
    </button>
  </div>
</aside>
