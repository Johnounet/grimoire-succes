<script lang="ts">
  // Page d'accueil. Bibliothèque vide (premier lancement) : bienvenue en trois étapes. Sinon, tableau
  // de bord ouvert depuis le panneau : bilan, jeux les plus proches du 100 %, derniers succès débloqués.
  import { rarityText } from "./filters";
  import {
    api,
    APP_NAME,
    APP_TAGLINE,
    formatAgo,
    formatPlaytime,
    isComplete,
    localImage,
    percent,
    type GameSummary,
    type RecentUnlock,
  } from "./library";
  import { appear } from "./motion.svelte";

  let {
    games,
    imagesDir,
    loadError = "",
    onopen,
    onadd,
    onimport,
  }: {
    games: GameSummary[];
    imagesDir: string;
    loadError?: string;
    onopen: (appid: number) => void;
    onadd: () => void;
    onimport: () => void;
  } = $props();

  const NEAR = 4;
  const RECENT = 8;

  const unlocked = $derived(games.reduce((n, g) => n + g.unlocked, 0));
  const total = $derived(games.reduce((n, g) => n + g.total, 0));
  const completed = $derived(games.filter(isComplete).length);
  const playtime = $derived(games.reduce((n, g) => n + (g.playtime_minutes ?? 0), 0));
  /** Jeux en cours, du moins de succès restants au plus. */
  const near = $derived(
    games
      .filter((g) => !isComplete(g) && g.total > 0)
      .sort((a, b) => a.total - a.unlocked - (b.total - b.unlocked) || percent(b) - percent(a))
      .slice(0, NEAR),
  );

  let recent = $state<RecentUnlock[] | null>(null);
  $effect(() => {
    // Relu quand la bibliothèque change (coche, actualisation automatique, jeu supprimé)
    void games.map((g) => g.unlocked).join();
    if (games.length === 0) return;
    api
      .recentUnlocks(RECENT)
      .then((r) => (recent = r))
      .catch(() => (recent = []));
  });

  const nf = (n: number) => n.toLocaleString("fr-FR");
</script>

{#snippet cover(g: GameSummary)}
  {@const src = localImage(imagesDir, g.cover_file)}
  <div class="h-[72px] w-12 shrink-0 overflow-hidden rounded-md bg-raised ring-1 ring-line">
    {#if src}
      <img {src} alt="" class="h-full w-full object-cover {g.cover_kind === 'header' ? 'object-left' : ''}" />
    {/if}
  </div>
{/snippet}

{#if games.length === 0}
  <!-- Premier lancement -->
  <main class="flex min-w-0 flex-1 items-center justify-center overflow-y-auto p-8" {@attach appear}>
    <div class="w-full max-w-3xl text-center">
      <div class="text-5xl" aria-hidden="true">🏆</div>
      <h1 class="mt-3 text-3xl font-bold">Bienvenue dans {APP_NAME}</h1>
      <p class="mt-1 text-sm italic text-muted">{APP_TAGLINE}</p>

      {#if loadError}
        <p class="mt-6 text-sm text-danger">Impossible d'ouvrir la base de données : {loadError}</p>
      {:else}
        <ol class="mt-10 grid gap-4 text-left sm:grid-cols-3">
          {#each [
            { icon: "🎮", title: "Ajoutez un jeu", text: "Ses succès arrivent de Steam : ceux déjà obtenus sont cochés, les cachés sont floutés." },
            { icon: "📜", title: "Préparez votre guide", text: "Posez les succès en cartes et ajoutez-leur des blocs : notes, compteurs, chronos, listes, images, vidéos." },
            { icon: "🪟", title: "Jouez en mode compact", text: "Épinglez des cartes : une petite fenêtre reste par-dessus le jeu, pilotée au clavier sans quitter la partie." },
          ] as step, i (step.title)}
            <li class="rounded-xl border border-line bg-panel p-4">
              <div class="flex items-center gap-2">
                <span class="grid h-6 w-6 place-items-center rounded-full bg-accent text-xs font-bold text-on-accent">{i + 1}</span>
                <span class="text-lg" aria-hidden="true">{step.icon}</span>
              </div>
              <h2 class="mt-3 font-semibold">{step.title}</h2>
              <p class="mt-1 text-sm text-muted">{step.text}</p>
            </li>
          {/each}
        </ol>

        <div class="mt-8 flex flex-col items-center gap-2">
          <button
            onclick={onadd}
            class="rounded-lg bg-accent px-5 py-2.5 text-sm font-medium text-on-accent hover:brightness-110 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
          >
            Ajouter mon premier jeu
          </button>
          <button onclick={onimport} class="text-sm text-muted underline-offset-2 hover:text-fg hover:underline">
            ou importer un guide reçu…
          </button>
        </div>
      {/if}
    </div>
  </main>
{:else}
  <!-- Tableau de bord -->
  <main class="min-w-0 flex-1 overflow-y-auto" {@attach appear}>
    <header class="border-b border-line bg-gradient-to-b from-accent/10 to-transparent px-8 pt-10 pb-6">
      <p class="text-xs font-semibold tracking-[0.2em] text-accent uppercase">Accueil</p>
      <h1 class="mt-1 text-3xl font-bold">{nf(unlocked)} succès obtenus</h1>
      <p class="mt-1 text-sm text-muted">sur {nf(total)}, dans {games.length} jeu{games.length > 1 ? "x" : ""}</p>

      <dl class="mt-6 grid max-w-2xl grid-cols-2 gap-3 sm:grid-cols-4">
        {#each [
          { label: "Progression", value: `${total ? Math.floor((unlocked / total) * 100) : 0} %` },
          { label: "Succès restants", value: nf(total - unlocked) },
          { label: "Jeux en cours", value: nf(games.length - completed) },
          { label: "Temps de jeu", value: playtime ? formatPlaytime(playtime) : "—" },
        ] as s (s.label)}
          <div class="rounded-xl border border-line bg-panel px-4 py-3">
            <dt class="text-xs text-muted">{s.label}</dt>
            <dd class="mt-0.5 text-xl font-semibold">{s.value}</dd>
          </div>
        {/each}
      </dl>
    </header>

    <div class="grid gap-8 p-8 lg:grid-cols-2">
      <section aria-labelledby="home-near">
        <h2 id="home-near" class="text-sm font-semibold tracking-wide text-muted uppercase">Presque terminés</h2>
        {#if near.length === 0}
          <p class="mt-3 text-sm text-muted">Tous vos jeux sont à 100 %. Il est temps d'en ajouter un !</p>
        {:else}
          <ul class="mt-3 space-y-2">
            {#each near as g (g.appid)}
              {@const left = g.total - g.unlocked}
              {@const pct = percent(g)}
              <li>
                <button
                  onclick={() => onopen(g.appid)}
                  class="flex w-full items-center gap-4 rounded-xl border border-line bg-panel p-2.5 pr-4 text-left transition hover:bg-raised focus-visible:outline-2 focus-visible:outline-accent"
                >
                  {@render cover(g)}
                  <div class="min-w-0 flex-1">
                    <div class="truncate font-medium">{g.name}</div>
                    <div class="mt-0.5 text-sm text-muted">
                      Plus que <span class="font-semibold text-fg">{left}</span> succès · {g.unlocked} / {g.total}
                    </div>
                    <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-line" aria-hidden="true">
                      <div class="h-full rounded-full bg-accent" style:width="{pct}%"></div>
                    </div>
                  </div>
                  <span class="text-sm font-semibold tabular-nums">{pct} %</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      <section aria-labelledby="home-recent">
        <h2 id="home-recent" class="text-sm font-semibold tracking-wide text-muted uppercase">Derniers succès débloqués</h2>
        {#if recent && recent.length === 0}
          <p class="mt-3 text-sm text-muted">Aucun succès daté pour l'instant : ils s'afficheront ici dès vos prochains déblocages.</p>
        {:else if recent}
          <ul class="mt-3 divide-y divide-line rounded-xl border border-line bg-panel">
            {#each recent as a (a.id)}
              <li>
                <button
                  onclick={() => onopen(a.appid)}
                  title="Ouvrir {a.game_name}"
                  class="flex w-full items-center gap-3 px-3 py-2.5 text-left transition first:rounded-t-xl last:rounded-b-xl hover:bg-raised focus-visible:outline-2 focus-visible:outline-accent"
                >
                  {#if a.icon_url}
                    <img src={a.icon_url} alt="" class="h-10 w-10 shrink-0 rounded-md" loading="lazy" />
                  {:else}
                    <span class="grid h-10 w-10 shrink-0 place-items-center rounded-md bg-raised" aria-hidden="true">🏆</span>
                  {/if}
                  <div class="min-w-0 flex-1">
                    <div class="truncate text-sm font-medium">{a.name}</div>
                    <div class="truncate text-xs text-muted">{a.game_name} · {formatAgo(a.unlock_time)}</div>
                  </div>
                  {#if a.rarity != null}
                    <span class="text-xs font-semibold tabular-nums {rarityText(a.rarity)}">{a.rarity.toFixed(1)} %</span>
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>
  </main>
{/if}
