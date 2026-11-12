<script lang="ts">
  import { api, heroUrl, percent, sourceLabel, type AchievementRow, type GameSummary } from "./library";
  import AchievementList from "./AchievementList.svelte";
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

  // Valeurs dérivées : elles ne changent que si la valeur change, pas à chaque mise à jour de `game`
  // (une coche renvoie un nouveau `game`, qui ne doit pas recharger la liste).
  const appidKey = $derived(game.appid);
  const refreshedKey = $derived(game.refreshed_at);

  $effect(() => {
    const appid = appidKey;
    void refreshedKey; // recharge aussi après une actualisation
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

  /** Coche instantanée à l'écran, enregistrée ensuite ; annulée si l'enregistrement échoue. */
  async function toggle(a: AchievementRow) {
    const row = achievements.find((x) => x.id === a.id);
    if (!row) return;
    const before = { unlocked: row.unlocked, manual: row.manual, unlock_time: row.unlock_time };
    row.unlocked = !row.unlocked;
    row.manual = true;
    if (!row.unlocked) row.unlock_time = null;
    try {
      onchanged(await api.setUnlocked(row.id, row.unlocked));
    } catch (e) {
      Object.assign(row, before);
      error = `Enregistrement impossible : ${e}`;
    }
  }
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

    {#key game.appid}
      <AchievementList {achievements} ontoggle={toggle} />
    {/key}
  </div>
</section>
