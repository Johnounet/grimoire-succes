<script lang="ts">
  // Annonce d'une nouvelle version, en bas à droite : installation en un clic (version installée)
  // ou lien vers la page de téléchargement (version portable).
  import { pop } from "./motion.svelte";
  import { installUpdate, openReleasesPage, update } from "./update.svelte";

  const shown = $derived(
    update.info != null && !update.dismissed && ["available", "installing", "error"].includes(update.status),
  );
</script>

{#if shown && update.info}
  <div
    role="status"
    transition:pop={{ y: 12 }}
    class="fixed right-5 bottom-5 z-40 w-80 rounded-xl border border-accent/40 bg-raised p-4 text-sm shadow-2xl"
  >
    <div class="flex items-start gap-2">
      <div class="flex-1">
        <div class="font-semibold">Grimoire {update.info.version} est disponible</div>
        <p class="mt-1 text-muted">
          {#if update.status === "installing"}
            Téléchargement… Grimoire va se fermer puis redémarrer.
          {:else if update.info.canInstall}
            L'installation ferme Grimoire puis le relance. Vos données sont conservées.
          {:else}
            Téléchargez la nouvelle version et remplacez l'ancien fichier <span class="font-mono">Grimoire.exe</span> :
            vos données, dans le dossier <span class="font-mono">donnees</span>, sont conservées.
          {/if}
        </p>
        {#if update.status === "error"}
          <p class="mt-1 text-danger">Échec : {update.error}</p>
        {/if}
      </div>
      <button
        onclick={() => (update.dismissed = true)}
        disabled={update.status === "installing"}
        class="rounded px-1 text-muted hover:text-fg disabled:opacity-40"
        aria-label="Fermer">✕</button
      >
    </div>
    <div class="mt-3 flex flex-wrap items-center gap-2">
      {#if update.info.canInstall}
        <button
          onclick={installUpdate}
          disabled={update.status === "installing"}
          class="rounded-lg bg-accent px-3 py-1.5 font-medium text-on-accent hover:brightness-110 disabled:opacity-60"
        >
          {update.status === "installing" ? "Installation…" : "Installer et redémarrer"}
        </button>
        <button onclick={openReleasesPage} class="px-1 text-muted hover:text-fg hover:underline">Nouveautés</button>
      {:else}
        <button
          onclick={openReleasesPage}
          class="rounded-lg bg-accent px-3 py-1.5 font-medium text-on-accent hover:brightness-110"
        >
          Télécharger
        </button>
      {/if}
      <button
        onclick={() => (update.dismissed = true)}
        disabled={update.status === "installing"}
        class="ml-auto px-1 text-muted hover:text-fg disabled:opacity-40">Plus tard</button
      >
    </div>
  </div>
{/if}
