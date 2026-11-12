<script lang="ts">
  import { localImage, percent, type GameSummary } from "./library";
  import ProgressRing from "./ProgressRing.svelte";

  let {
    games,
    imagesDir,
    selected,
    onselect,
    onadd,
    onsettings,
  }: {
    games: GameSummary[];
    imagesDir: string;
    selected: number | null;
    onselect: (appid: number) => void;
    onadd: () => void;
    onsettings: () => void;
  } = $props();
</script>

<aside class="flex w-72 shrink-0 flex-col border-r border-line bg-panel">
  <div class="p-3">
    <button
      onclick={onadd}
      class="flex w-full items-center justify-center gap-2 rounded-lg bg-accent px-3 py-2 text-sm font-medium text-white transition hover:brightness-110 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
    >
      <span class="text-lg leading-none">+</span> Ajouter un jeu
    </button>
  </div>

  <nav class="flex-1 overflow-y-auto px-2 pb-3" aria-label="Jeux importés">
    {#if games.length === 0}
      <p class="px-3 py-6 text-center text-sm text-muted">Aucun jeu pour l'instant.</p>
    {/if}
    <ul class="space-y-1">
      {#each games as g (g.appid)}
        {@const pct = percent(g)}
        {@const cover = localImage(imagesDir, g.cover_file)}
        <li>
          <button
            onclick={() => onselect(g.appid)}
            aria-current={selected === g.appid ? "page" : undefined}
            class="group flex w-full items-center gap-3 rounded-lg p-1.5 text-left transition
              hover:bg-raised focus-visible:outline-2 focus-visible:outline-accent
              {selected === g.appid ? 'bg-raised' : ''}"
          >
            <div
              class="h-[60px] w-10 shrink-0 overflow-hidden rounded-md bg-raised ring-1
                {pct >= 100 ? 'ring-gold' : 'ring-line'}"
            >
              {#if cover}
                <img
                  src={cover}
                  alt=""
                  class="h-full w-full {g.cover_kind === 'header' ? 'object-cover object-left' : 'object-cover'}"
                />
              {/if}
            </div>
            <div class="min-w-0 flex-1">
              <div class="truncate text-sm font-medium">{g.name}</div>
              <div class="text-xs text-muted">{g.unlocked} / {g.total} · {pct} %</div>
            </div>
            <ProgressRing value={pct} />
          </button>
        </li>
      {/each}
    </ul>
  </nav>

  <div class="border-t border-line p-2">
    <button
      onclick={onsettings}
      class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-muted transition hover:bg-raised hover:text-fg"
    >
      <span aria-hidden="true">⚙</span> Réglages
    </button>
  </div>
</aside>
