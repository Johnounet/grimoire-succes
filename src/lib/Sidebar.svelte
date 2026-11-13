<script lang="ts">
  // Panneau de gauche : jeux importés. Repliable : il ne montre alors que les jaquettes (avec un
  // liseré de progression), pour garder le changement de jeu à portée de clic.
  // Un jeu terminé à 100 % passe en version dorée (bordure, reflet, badge « 100 % »).
  import { isComplete, localImage, percent, type GameSummary } from "./library";
  import ProgressRing from "./ProgressRing.svelte";
  import { setSidebarOpen, settings } from "./settings.svelte";

  let {
    games,
    imagesDir,
    selected,
    showcase,
    onselect,
    onshowcase,
    onadd,
    onsettings,
    onabout,
  }: {
    games: GameSummary[];
    imagesDir: string;
    selected: number | null;
    /** La vitrine des trophées est affichée. */
    showcase: boolean;
    onselect: (appid: number) => void;
    onshowcase: () => void;
    onadd: () => void;
    onsettings: () => void;
    onabout: () => void;
  } = $props();

  const open = $derived(settings.sidebarOpen);
  const trophies = $derived(games.filter(isComplete).length);
</script>

{#snippet cover(g: GameSummary, gold: boolean, i: number)}
  {@const src = localImage(imagesDir, g.cover_file)}
  <div
    class="h-[60px] w-10 shrink-0 overflow-hidden rounded-md bg-raised {gold ? 'gold-frame gold-sm' : 'ring-1 ring-line'}"
    style:--sheen-delay={gold ? `${(i % 5) * 0.9}s` : undefined}
  >
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
      class="flex items-center justify-center gap-2 rounded-lg bg-accent py-2 text-sm font-medium text-on-accent transition hover:brightness-110 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent
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
    <!-- Vitrine des trophées -->
    <button
      onclick={onshowcase}
      aria-current={showcase ? "page" : undefined}
      title={open ? undefined : `Vitrine des trophées · ${trophies}`}
      aria-label={open ? undefined : "Vitrine des trophées"}
      class="mb-2 flex w-full items-center gap-3 rounded-lg p-1.5 text-left transition hover:bg-raised focus-visible:outline-2 focus-visible:outline-accent
        {showcase ? 'bg-raised' : ''} {open ? '' : 'flex-col gap-0.5'}"
    >
      <span class="grid h-10 w-10 shrink-0 place-items-center rounded-md bg-gold/15 text-lg" aria-hidden="true">🏆</span>
      {#if open}
        <span class="min-w-0 flex-1 truncate text-sm font-medium">Vitrine des trophées</span>
        {#if trophies}<span class="rounded-full bg-gold/20 px-2 text-xs font-semibold text-gold">{trophies}</span>{/if}
      {:else if trophies}
        <span class="text-[10px] font-semibold text-gold">{trophies}</span>
      {/if}
    </button>

    <ul class="space-y-1">
      {#each games as g, i (g.appid)}
        {@const pct = percent(g)}
        {@const gold = isComplete(g)}
        <li>
          <button
            onclick={() => onselect(g.appid)}
            aria-current={selected === g.appid && !showcase ? "page" : undefined}
            title={open ? undefined : `${g.name} · ${g.unlocked} / ${g.total} · ${gold ? "100" : pct} %`}
            aria-label={open ? undefined : g.name}
            class="group flex w-full items-center gap-3 rounded-lg p-1.5 text-left transition
              hover:bg-raised focus-visible:outline-2 focus-visible:outline-accent
              {selected === g.appid && !showcase ? 'bg-raised' : ''} {gold ? 'bg-gradient-to-r from-gold/10 to-transparent' : ''}
              {open ? '' : 'flex-col gap-1'}"
          >
            {@render cover(g, gold, i)}
            {#if open}
              <div class="min-w-0 flex-1">
                <div class="truncate text-sm font-medium">{g.name}</div>
                <div class="text-xs {gold ? 'text-gold/80' : 'text-muted'}">{g.unlocked} / {g.total} · {gold ? 100 : pct} %</div>
              </div>
              {#if gold}
                <span class="rounded-full bg-gold px-1.5 py-0.5 text-[10px] font-bold text-bg">100 %</span>
              {:else}
                <ProgressRing value={pct} />
              {/if}
            {:else}
              <!-- Progression sous la jaquette -->
              <div class="h-1 w-10 overflow-hidden rounded-full bg-line" aria-hidden="true">
                <div class="h-full {gold ? 'bg-gold' : 'bg-accent'}" style:width="{gold ? 100 : pct}%"></div>
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
