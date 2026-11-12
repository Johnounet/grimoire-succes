<script lang="ts">
  import { api, heroUrl, percent, sourceLabel, type AchievementRow, type GameSummary } from "./library";
  import ProgressRing from "./ProgressRing.svelte";

  let {
    game,
    onchanged,
    ondeleted,
  }: {
    game: GameSummary;
    onchanged: (g: GameSummary) => void;
    ondeleted: (appid: number) => void;
  } = $props();

  let achievements = $state<AchievementRow[]>([]);
  let refreshing = $state(false);
  let confirmDelete = $state(false);
  let error = $state("");
  let heroOk = $state(true);

  $effect(() => {
    const appid = game.appid;
    const stamp = game.refreshed_at; // recharge aussi après une actualisation
    void stamp;
    heroOk = true;
    confirmDelete = false;
    error = "";
    api.achievements(appid).then((a) => {
      if (appid === game.appid) achievements = a;
    });
  });

  const pct = $derived(percent(game));

  async function refresh() {
    refreshing = true;
    error = "";
    try {
      onchanged(await api.importGame(game.appid));
    } catch (e) {
      error = `Actualisation impossible : ${e}`;
    } finally {
      refreshing = false;
    }
  }

  async function remove() {
    try {
      await api.deleteGame(game.appid);
      ondeleted(game.appid);
    } catch (e) {
      error = `Suppression impossible : ${e}`;
    }
  }

  const rarityClass = (r: number | null) =>
    r == null ? "text-muted" : r < 5 ? "text-gold" : r < 20 ? "text-rare" : "text-muted";

  const fmtDate = (t: number | null) =>
    t ? new Date(t * 1000).toLocaleDateString("fr-FR", { day: "numeric", month: "short", year: "numeric" }) : "";
</script>

<section class="flex min-w-0 flex-1 flex-col overflow-y-auto">
  <!-- Bannière -->
  <header class="relative h-56 shrink-0 overflow-hidden bg-raised">
    {#if heroOk}
      <img
        src={heroUrl(game.appid)}
        alt=""
        class="absolute inset-0 h-full w-full object-cover"
        onerror={() => (heroOk = false)}
      />
    {/if}
    <div class="absolute inset-0 bg-gradient-to-t from-bg via-bg/50 to-transparent"></div>
    <div class="absolute inset-x-0 bottom-0 flex items-end gap-4 px-6 pb-5">
      <div class="min-w-0 flex-1">
        <h1 class="truncate text-3xl font-bold drop-shadow">{game.name}</h1>
        <p class="mt-1 text-sm text-fg/80">
          <span class="font-semibold {pct >= 100 ? 'text-gold' : ''}">{game.unlocked} / {game.total}</span>
          succès · {pct} %
        </p>
      </div>
      <ProgressRing value={pct} size={48} />
      <button
        onclick={refresh}
        disabled={refreshing}
        class="rounded-lg border border-line bg-panel/80 px-3 py-1.5 text-sm backdrop-blur transition hover:bg-raised disabled:opacity-60"
      >
        {refreshing ? "Actualisation…" : "Actualiser"}
      </button>
      <button
        onclick={() => (confirmDelete = true)}
        class="rounded-lg border border-line bg-panel/80 px-3 py-1.5 text-sm text-muted backdrop-blur transition hover:text-danger"
        aria-label="Supprimer le jeu"
        title="Supprimer le jeu"
      >
        Supprimer
      </button>
    </div>
  </header>

  <div class="space-y-3 px-6 py-4">
    {#if confirmDelete}
      <div class="flex flex-wrap items-center gap-3 rounded-lg border border-danger/40 bg-danger/10 p-3 text-sm">
        <span class="flex-1">
          Supprimer « {game.name} » de la bibliothèque ? Ses succès importés seront effacés.
          <span class="text-muted">(L'export des guides avant suppression arrivera avec les guides.)</span>
        </span>
        <button onclick={() => (confirmDelete = false)} class="rounded-md px-3 py-1 hover:bg-raised">Annuler</button>
        <button onclick={remove} class="rounded-md bg-danger px-3 py-1 font-medium text-white">Supprimer</button>
      </div>
    {/if}

    {#if error}
      <p class="rounded-lg bg-danger/10 p-3 text-sm text-danger">{error}</p>
    {/if}

    <p class="text-xs text-muted">
      Importé depuis le {sourceLabel[game.source]}
      {#if !game.has_api_names}· certains ID techniques manquent (re-lock indisponible pour ceux-là){/if}
    </p>

    <!-- Liste simple ; la liste complète avec filtres arrive à l'étape 4 -->
    <ul class="divide-y divide-line rounded-xl border border-line bg-panel">
      {#each achievements as a (a.id)}
        {@const icon = a.unlocked ? a.icon_url : (a.icon_gray_url ?? a.icon_url)}
        <li class="flex items-center gap-3 p-3 {a.removed ? 'opacity-50' : ''}">
          <div class="h-12 w-12 shrink-0 overflow-hidden rounded-md bg-raised {a.unlocked ? 'shadow-[0_0_12px_-2px] shadow-accent/50' : ''}">
            {#if icon}<img src={icon} alt="" loading="lazy" class="h-full w-full {a.unlocked ? '' : 'grayscale'}" />{/if}
          </div>
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="truncate font-medium {a.unlocked ? '' : 'text-fg/80'}">{a.name}</span>
              {#if a.removed}<span class="rounded bg-raised px-1.5 text-[10px] text-muted">retiré</span>{/if}
              {#if a.dlc_appid}<span class="rounded bg-raised px-1.5 text-[10px] text-muted">DLC</span>{/if}
            </div>
            <p class="truncate text-sm text-muted {a.hidden && !a.unlocked ? 'blur-[5px] select-none' : ''}">
              {a.description || "—"}
            </p>
          </div>
          <div class="shrink-0 text-right text-xs">
            {#if a.rarity != null}
              <div class="font-semibold {rarityClass(a.rarity)}">{a.rarity.toFixed(1)} %</div>
            {/if}
            {#if a.unlocked}
              <div class="text-ok">✓ {fmtDate(a.unlock_time)}</div>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  </div>
</section>
