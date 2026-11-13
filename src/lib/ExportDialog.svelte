<script lang="ts">
  // Export des guides d'un jeu dans un fichier : vierge pour le partager, ou avec l'état des blocs (sauvegarde).
  // Le fichier ne contient jamais la progression Steam, le compte ni les réglages (voir guides.rs).
  import { flushBlocks, showToast } from "./blocks/store.svelte";
  import { api } from "./library";
  import { backdrop, pop } from "./motion.svelte";

  let {
    appid,
    gameName,
    onclose,
    ondone,
  }: {
    appid: number;
    gameName: string;
    onclose: () => void;
    /** Fichier enregistré (suppression d'un jeu : on peut alors confirmer). */
    ondone?: () => void;
  } = $props();

  let keepState = $state(false);
  let busy = $state(false);
  let error = $state("");

  async function run() {
    busy = true;
    error = "";
    try {
      await flushBlocks();
      const path = await api.guideExport(appid, keepState);
      if (path == null) return; // annulé dans la fenêtre d'enregistrement
      showToast(`Guide exporté : ${path}`);
      ondone?.();
      onclose();
    } catch (e) {
      error = `Export impossible : ${e}`;
    } finally {
      busy = false;
    }
  }
</script>

<div
  class="fixed inset-0 z-50 grid place-items-center bg-black/60 p-4"
  role="presentation"
  transition:backdrop
  onclick={(e) => e.target === e.currentTarget && onclose()}
  onkeydown={(e) => e.key === "Escape" && onclose()}
>
  <div role="dialog" aria-modal="true" aria-labelledby="export-title" transition:pop class="w-full max-w-md rounded-2xl border border-line bg-panel p-5 shadow-2xl">
    <h2 id="export-title" class="text-lg font-semibold">Exporter le guide de « {gameName} »</h2>
    <p class="mt-1 text-sm text-muted">
      Un fichier avec vos cartes, cartes libres, blocs (images comprises) et étiquettes. Jamais votre progression
      Steam, votre compte ni vos réglages : celui qui l'importe garde ses propres succès.
    </p>

    <label class="mt-4 flex cursor-pointer items-start gap-2.5 rounded-lg border border-line bg-bg p-3 text-sm">
      <input type="checkbox" bind:checked={keepState} class="mt-0.5 accent-accent" />
      <span>
        Garder l'état des compteurs, listes et chronos
        <span class="block text-xs text-muted">
          Pour une sauvegarde. Décoché, le guide est vierge pour le partager : compteurs à zéro, listes décochées,
          chronos sans meilleur temps, cartes « à faire ».
        </span>
      </span>
    </label>

    {#if error}<p class="mt-3 text-sm text-danger">{error}</p>{/if}

    <div class="mt-5 flex justify-end gap-2">
      <button onclick={onclose} class="rounded-lg px-3 py-1.5 text-sm text-muted hover:bg-raised hover:text-fg">Annuler</button>
      <button onclick={run} disabled={busy} class="rounded-lg bg-accent px-3 py-1.5 text-sm font-medium text-white hover:brightness-110 disabled:opacity-60">
        {busy ? "Export…" : "Exporter…"}
      </button>
    </div>
  </div>
</div>
