<script lang="ts">
  // Assistant de re-lock : prépare la commande console Steam, que le joueur lance lui-même.
  // Le logiciel n'exécute jamais rien dans Steam (règle du projet) : il affiche, copie, ouvre la console.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { AchievementRow } from "./library";

  let {
    appid,
    gameName,
    target,
    onclose,
    onlock,
  }: {
    appid: number;
    gameName: string;
    /** Un succès, ou null pour le reset complet du jeu. */
    target: AchievementRow | null;
    onclose: () => void;
    /** Décoche dans le logiciel le succès (ou tous ceux du jeu). */
    onlock: () => Promise<void>;
  } = $props();

  const full = $derived(target == null);
  const command = $derived(
    full ? `reset_all_stats ${appid}` : target?.api_name ? `achievement_clear ${appid} ${target.api_name}` : null,
  );
  const steps = $derived(
    [
      "Fermer le jeu.",
      "Ouvrir la console Steam et y coller la commande, puis Entrée.",
      full ? "Désactiver Steam Cloud pour ce jeu (Propriétés → Général), sinon les anciennes sauvegardes reviennent." : null,
      "Supprimer les sauvegardes locales du jeu (leur emplacement est indiqué sur PCGamingWiki).",
      full ? "Relancer le jeu et vérifier que les succès sont bien reverrouillés." : "Relancer le jeu et vérifier que le succès est bien reverrouillé.",
    ].filter((s): s is string => s != null),
  );
  let done = $state<boolean[]>([]);
  let copied = $state(false);
  let error = $state("");
  let busy = $state(false);

  async function copy() {
    if (!command) return;
    try {
      await navigator.clipboard.writeText(command);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch (e) {
      error = `Copie impossible : ${e}`;
    }
  }

  async function open(url: string) {
    try {
      await openUrl(url);
    } catch (e) {
      error = `Ouverture impossible : ${e}`;
    }
  }

  async function confirm() {
    busy = true;
    try {
      await onlock();
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div
  class="fixed inset-0 z-50 grid place-items-center bg-black/60 p-4"
  role="presentation"
  onclick={(e) => e.target === e.currentTarget && onclose()}
  onkeydown={(e) => e.key === "Escape" && onclose()}
>
  <div role="dialog" aria-modal="true" aria-labelledby="relock-title" class="w-full max-w-lg rounded-2xl border border-line bg-panel p-5 shadow-2xl">
    <h2 id="relock-title" class="text-lg font-semibold">
      {full ? `Reset complet de « ${gameName} »` : `Reverrouiller « ${target?.name} »`}
    </h2>
    <p class="mt-1 text-sm text-muted">
      Le logiciel ne touche jamais à vos succès Steam : il prépare la commande, vous la lancez vous-même dans la console Steam.
    </p>

    {#if !command}
      <p class="mt-4 rounded-lg border border-line bg-bg p-3 text-sm text-muted">
        L'ID technique de ce succès est inconnu (jeu importé depuis la page publique Steam) : la commande ne peut pas
        être préparée. Lancez le jeu une fois sur ce PC puis cliquez sur « Actualiser » pour le récupérer.
      </p>
    {:else}
      <div class="mt-4 flex items-center gap-2">
        <code class="min-w-0 flex-1 truncate rounded-lg border border-line bg-bg px-3 py-2 font-mono text-sm select-all">{command}</code>
        <button onclick={copy} class="rounded-lg bg-accent px-3 py-2 text-sm font-medium text-white hover:brightness-110">
          {copied ? "Copiée ✓" : "Copier"}
        </button>
      </div>
      <button onclick={() => open("steam://open/console")} class="mt-2 text-sm text-accent hover:underline">
        Ouvrir la console Steam ↗
      </button>

      <ol class="mt-4 space-y-1.5">
        {#each steps as s, i (s)}
          <li>
            <label class="flex cursor-pointer items-start gap-2 text-sm">
              <input type="checkbox" bind:checked={done[i]} class="mt-0.5 accent-accent" />
              <span class={done[i] ? "text-muted line-through" : ""}>
                {i + 1}. {s}
                {#if s.includes("PCGamingWiki")}
                  <button
                    onclick={(e) => {
                      e.preventDefault();
                      open(`https://www.pcgamingwiki.com/api/appid.php?appid=${appid}`);
                    }}
                    class="text-accent no-underline hover:underline">ouvrir la page ↗</button
                  >
                {/if}
              </span>
            </label>
          </li>
        {/each}
      </ol>

      <p class="mt-4 rounded-lg border border-danger/40 bg-danger/10 p-3 text-xs text-fg/90">
        ⚠ Action irréversible. Certains jeux redébloquent les succès depuis leur sauvegarde ou leurs serveurs.
      </p>
    {/if}

    {#if error}<p class="mt-3 text-sm text-danger">{error}</p>{/if}

    <div class="mt-5 flex justify-end gap-2">
      <button onclick={onclose} class="rounded-lg px-3 py-1.5 text-sm text-muted hover:bg-raised hover:text-fg">Fermer</button>
      {#if command}
        <button
          onclick={confirm}
          disabled={busy}
          title="Ne touche pas aux guides ni aux étiquettes"
          class="rounded-lg border border-line px-3 py-1.5 text-sm hover:bg-raised disabled:opacity-60"
        >
          {full ? "C'est fait : décocher tous les succès" : "C'est fait : décocher ce succès"}
        </button>
      {/if}
    </div>
  </div>
</div>
