<script lang="ts">
  // Vitrine des trophées : les jeux terminés à 100 %, jaquettes dorées, du plus récent au plus ancien.
  import { appear } from "./motion.svelte";
  import { formatDate, formatPlaytime, isComplete, localImage, type GameSummary } from "./library";

  let {
    games,
    imagesDir,
    onopen,
    oncelebrate,
  }: {
    games: GameSummary[];
    imagesDir: string;
    onopen: (appid: number) => void;
    oncelebrate: (appid: number) => void;
  } = $props();

  const done = $derived(games.filter(isComplete).sort((a, b) => b.completed_at! - a.completed_at!));
  const totalAchievements = $derived(done.reduce((n, g) => n + g.total, 0));
</script>

<main class="min-w-0 flex-1 overflow-y-auto" {@attach appear}>
  <header class="border-b border-line bg-gradient-to-b from-gold/10 to-transparent px-8 pt-10 pb-6">
    <p class="text-xs font-semibold tracking-[0.2em] text-gold uppercase">Vitrine des trophées</p>
    <h1 class="mt-1 text-3xl font-bold">
      {done.length === 0 ? "Aucun 100 % pour l'instant" : `${done.length} jeu${done.length > 1 ? "x" : ""} terminé${done.length > 1 ? "s" : ""}`}
    </h1>
    {#if done.length}
      <p class="mt-1 text-sm text-muted">{totalAchievements.toLocaleString("fr-FR")} succès obtenus dans ces jeux</p>
    {:else}
      <p class="mt-1 text-sm text-muted">Chaque jeu terminé à 100 % viendra prendre sa place ici, avec sa jaquette dorée.</p>
    {/if}
  </header>

  <ul class="grid grid-cols-[repeat(auto-fill,minmax(10rem,1fr))] gap-x-6 gap-y-8 p-8">
    {#each done as g, i (g.appid)}
      {@const src = localImage(imagesDir, g.cover_file)}
      <li class="group flex flex-col">
        <button
          onclick={() => onopen(g.appid)}
          title="Ouvrir {g.name}"
          class="gold-frame rounded-xl bg-raised transition group-hover:-translate-y-1 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-gold"
          style:aspect-ratio="2 / 3"
          style:--sheen-delay="{(i % 6) * 0.8}s"
        >
          {#if src}
            <img {src} alt="" class="h-full w-full object-cover {g.cover_kind === 'header' ? 'object-left' : ''}" />
          {:else}
            <span class="grid h-full place-items-center px-3 text-center text-sm font-semibold">{g.name}</span>
          {/if}
          <span class="absolute top-2 right-2 z-10 rounded-full bg-gold px-2 py-0.5 text-[11px] font-bold text-bg shadow">100 %</span>
        </button>
        <div class="mt-3 flex items-start gap-1">
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-medium" title={g.name}>{g.name}</div>
            <div class="text-xs text-muted">
              {formatDate(g.completed_at!)}{#if g.playtime_minutes}{" · "}{formatPlaytime(g.playtime_minutes)}{/if}
            </div>
          </div>
          <button
            onclick={() => oncelebrate(g.appid)}
            title="Revoir la célébration"
            aria-label="Revoir la célébration de {g.name}"
            class="rounded px-1.5 py-0.5 text-muted opacity-0 transition group-focus-within:opacity-100 group-hover:opacity-100 hover:bg-raised hover:text-gold"
          >
            🎉
          </button>
        </div>
      </li>
    {/each}
  </ul>
</main>
