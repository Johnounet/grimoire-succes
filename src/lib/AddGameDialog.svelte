<script lang="ts">
  import { onMount } from "svelte";
  import { api, type GameSummary, type SearchHit } from "./library";

  let { onclose, onimported }: { onclose: () => void; onimported: (g: GameSummary) => void } = $props();

  let query = $state("");
  let results = $state<SearchHit[]>([]);
  let local = $state<SearchHit[]>([]);
  let searching = $state(false);
  let error = $state("");
  let importing = $state<number | null>(null);
  let input: HTMLInputElement;
  let dialog: HTMLDialogElement;

  onMount(async () => {
    dialog.showModal();
    input.focus();
    try {
      local = await api.localSuggestions();
    } catch {
      local = [];
    }
  });

  // Les jeux de ce PC, filtrés localement pendant la frappe
  const localShown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return (q ? local.filter((h) => h.name.toLowerCase().includes(q)) : local).slice(0, 60);
  });

  // Recherche magasin avec un petit délai pour ne pas interroger Steam à chaque touche
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;
  $effect(() => {
    const q = query.trim();
    clearTimeout(timer);
    error = "";
    if (q.length < 2) {
      results = [];
      searching = false;
      return;
    }
    searching = true;
    const mine = ++seq;
    timer = setTimeout(async () => {
      try {
        const r = await api.search(q);
        if (mine === seq) results = r;
      } catch (e) {
        if (mine === seq) {
          results = [];
          error = String(e);
        }
      } finally {
        if (mine === seq) searching = false;
      }
    }, 350);
  });

  async function importHit(hit: SearchHit) {
    if (importing !== null) return;
    importing = hit.appid;
    error = "";
    try {
      const g = await api.importGame(hit.appid);
      onimported(g);
    } catch (e) {
      error = `Import de « ${hit.name} » impossible : ${e}`;
    } finally {
      importing = null;
    }
  }
</script>

{#snippet row(hit: SearchHit)}
  <li>
    <button
      onclick={() => importHit(hit)}
      disabled={importing !== null}
      class="flex w-full items-center gap-3 rounded-lg p-2 text-left transition hover:bg-raised
        focus-visible:outline-2 focus-visible:outline-accent disabled:cursor-wait"
    >
      <img
        src={hit.capsule_url}
        alt=""
        loading="lazy"
        class="h-[45px] w-[120px] shrink-0 rounded bg-raised object-cover"
        onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = "hidden")}
      />
      <div class="min-w-0 flex-1">
        <div class="truncate text-sm font-medium">{hit.name}</div>
        <div class="flex gap-2 text-xs text-muted">
          <span>AppID {hit.appid}</span>
          {#if hit.on_this_pc}<span class="text-ok">· sur ce PC</span>{/if}
        </div>
      </div>
      <span class="shrink-0 text-xs font-medium {hit.in_library ? 'text-muted' : 'text-accent'}">
        {#if importing === hit.appid}
          Import…
        {:else if hit.in_library}
          Actualiser
        {:else}
          Importer
        {/if}
      </span>
    </button>
  </li>
{/snippet}

<dialog
  bind:this={dialog}
  onclose={onclose}
  class="dialog-pop m-auto flex max-h-[80vh] w-[640px] max-w-[92vw] flex-col overflow-hidden rounded-xl border border-line bg-panel p-0 text-fg shadow-2xl backdrop:bg-black/60"
>
  <div class="border-b border-line p-4">
    <div class="mb-3 flex items-center justify-between">
      <h2 class="text-base font-semibold">Ajouter un jeu</h2>
      <button onclick={() => dialog.close()} class="rounded px-2 text-muted hover:text-fg" aria-label="Fermer">✕</button>
    </div>
    <input
      bind:this={input}
      bind:value={query}
      placeholder="Nom du jeu, AppID ou lien Steam"
      class="w-full rounded-lg border border-line bg-bg px-3 py-2 text-sm outline-none focus:border-accent"
    />
    {#if error}
      <p class="mt-2 text-sm text-danger">{error}</p>
    {/if}
  </div>

  <div class="flex-1 overflow-y-auto p-2">
    {#if query.trim().length >= 2}
      {#if searching && results.length === 0}
        <p class="p-4 text-sm text-muted">Recherche…</p>
      {:else if !searching && results.length === 0 && !error}
        <p class="p-4 text-sm text-muted">Aucun résultat.</p>
      {/if}
      <ul>
        {#each results as hit (hit.appid)}{@render row(hit)}{/each}
      </ul>
    {:else if local.length > 0}
      <h3 class="px-2 pt-1 pb-2 text-xs font-semibold tracking-wide text-muted uppercase">
        Jeux déjà lancés sur ce PC ({local.length})
      </h3>
      <ul>
        {#each localShown as hit (hit.appid)}{@render row(hit)}{/each}
      </ul>
      {#if local.length > localShown.length}
        <p class="p-2 text-center text-xs text-muted">Tapez un nom pour trouver les autres.</p>
      {/if}
    {:else}
      <p class="p-4 text-sm text-muted">Tapez le nom d'un jeu, son AppID (ex. 620) ou collez le lien de sa page Steam.</p>
    {/if}
  </div>
</dialog>
