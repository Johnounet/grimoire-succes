<script lang="ts">
  // Réglages › Sauvegardes : copies de sécurité des données (backup.rs). Une automatique par jour,
  // les 7 dernières gardées ; sauvegarde à la demande, restauration (l'état remplacé est d'abord
  // sauvegardé), suppression, ouverture du dossier.
  import { onMount } from "svelte";
  import { flushBlocks } from "./blocks/store.svelte";
  import { api, type BackupInfo } from "./library";
  import { send } from "./sync";

  let backups = $state<BackupInfo[]>([]);
  let busy = $state(false);
  let error = $state("");
  /** Action en attente de confirmation sur une sauvegarde. */
  let confirm = $state<{ id: string; action: "restore" | "delete" } | null>(null);

  const KIND: Record<BackupInfo["kind"], string> = {
    auto: "automatique",
    manuel: "manuelle",
    "avant-restauration": "avant restauration",
  };

  const when = (t: number) =>
    new Date(t * 1000).toLocaleString("fr-FR", { weekday: "short", day: "numeric", month: "long", hour: "2-digit", minute: "2-digit" });
  const size = (b: number) => (b < 1024 * 1024 ? `${Math.max(1, Math.round(b / 1024))} Ko` : `${(b / 1024 / 1024).toFixed(1).replace(".", ",")} Mo`);
  const games = (n: number) => `${n} jeu${n > 1 ? "x" : ""}`;

  async function load() {
    try {
      backups = await api.backups();
    } catch (e) {
      error = String(e);
    }
  }
  onMount(load);

  async function run(task: () => Promise<unknown>) {
    busy = true;
    error = "";
    try {
      await task();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const createNow = () => run(async () => {
    await flushBlocks();
    await api.backupCreate();
    await load();
  });

  const remove = (id: string) => run(async () => {
    confirm = null;
    await api.backupDelete(id);
    await load();
  });

  const restore = (id: string) => run(async () => {
    confirm = null;
    // Modifications en attente enregistrées avant : elles font partie de l'état mis de côté
    await flushBlocks();
    await api.backupRestore(id);
    // Tout est rechargé depuis la base restaurée, dans les deux fenêtres
    send("data-restored", {});
    location.reload();
  });
</script>

<section aria-labelledby="s-backups">
  <h3 id="s-backups" class="text-sm font-semibold tracking-wide text-muted uppercase">Sauvegardes</h3>
  <p class="mt-1 text-xs text-muted">
    Une copie automatique par jour de vos guides, coches, étiquettes et réglages ; les 7 dernières sont gardées.
    Restaurer remet les données d'une sauvegarde, après avoir mis de côté celles d'aujourd'hui.
  </p>

  <div class="mt-3 flex flex-wrap gap-2">
    <button
      onclick={createNow}
      disabled={busy}
      class="rounded-lg bg-accent px-3 py-1.5 text-sm font-medium text-on-accent hover:brightness-110 disabled:opacity-50"
    >
      Sauvegarder maintenant
    </button>
    <button
      onclick={() => run(api.backupsOpenFolder)}
      class="rounded-lg border border-line px-3 py-1.5 text-sm hover:bg-raised"
    >
      Ouvrir le dossier
    </button>
  </div>

  {#if error}<p class="mt-2 text-sm text-danger">{error}</p>{/if}

  {#if backups.length === 0}
    <p class="mt-3 text-sm text-muted">Aucune sauvegarde pour l'instant. La première se fera dès qu'un jeu sera ajouté.</p>
  {:else}
    <ul class="mt-3 divide-y divide-line rounded-lg border border-line">
      {#each backups as b (b.id)}
        <li class="px-3 py-2 text-sm">
          <div class="flex items-center gap-3">
            <div class="min-w-0 flex-1">
              <div class="font-medium first-letter:uppercase">{when(b.created_at)}</div>
              <div class="text-xs text-muted">{KIND[b.kind]} · {games(b.games)} · {size(b.size)}</div>
            </div>
            <button
              onclick={() => (confirm = { id: b.id, action: "restore" })}
              disabled={busy}
              class="rounded-md border border-line px-2 py-1 text-xs hover:bg-raised disabled:opacity-50"
            >
              Restaurer
            </button>
            <button
              onclick={() => (confirm = { id: b.id, action: "delete" })}
              disabled={busy}
              title="Supprimer cette sauvegarde"
              aria-label="Supprimer la sauvegarde du {when(b.created_at)}"
              class="rounded-md px-2 py-1 text-xs text-muted hover:bg-raised hover:text-danger disabled:opacity-50"
            >
              ✕
            </button>
          </div>
          {#if confirm?.id === b.id}
            <div class="mt-2 flex flex-wrap items-center gap-2 rounded-lg border p-2 text-xs
              {confirm.action === 'delete' ? 'border-danger/40 bg-danger/10' : 'border-accent/40 bg-accent/10'}">
              <span class="min-w-0 flex-1">
                {#if confirm.action === "restore"}
                  Remplacer vos données actuelles par celles de cette sauvegarde ? Elles seront d'abord sauvegardées.
                {:else}
                  Supprimer définitivement cette sauvegarde ?
                {/if}
              </span>
              <button onclick={() => (confirm = null)} class="rounded-md px-2 py-0.5 hover:bg-raised">Annuler</button>
              {#if confirm.action === "restore"}
                <button onclick={() => restore(b.id)} class="rounded-md bg-accent px-2 py-0.5 font-medium text-on-accent">Restaurer</button>
              {:else}
                <button onclick={() => remove(b.id)} class="rounded-md bg-danger px-2 py-0.5 font-medium text-white">Supprimer</button>
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
