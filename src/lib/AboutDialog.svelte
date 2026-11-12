<script lang="ts">
  // « À propos » : version, dépôt, contact, et transparence sur la façon dont le logiciel a été écrit.
  import { backdrop, pop } from "./motion.svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";

  /** Adresse du dépôt GitHub. null tant qu'il n'est pas publié : le lien est alors remplacé par « bientôt ». */
  const GITHUB_URL = null as string | null;
  const DISCORD = "Johnounet";

  let { onclose }: { onclose: () => void } = $props();

  let version = $state("");
  let copied = $state(false);

  onMount(async () => {
    version = await getVersion().catch(() => "dev");
  });

  function open(url: string) {
    openUrl(url).catch(() => window.open(url, "_blank"));
  }

  async function copyDiscord() {
    try {
      await navigator.clipboard.writeText(DISCORD);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      /* presse-papiers indisponible : le pseudo reste lisible */
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
  <div
    role="dialog"
    aria-modal="true"
    aria-labelledby="about-title" transition:pop
    class="flex max-h-[90vh] w-full max-w-md flex-col rounded-2xl border border-line bg-panel shadow-2xl"
  >
    <header class="flex items-center border-b border-line px-5 py-3">
      <h2 id="about-title" class="flex-1 text-lg font-semibold">À propos</h2>
      <button onclick={onclose} class="rounded px-2 py-1 text-muted hover:bg-raised hover:text-fg" aria-label="Fermer">✕</button>
    </header>

    <div class="space-y-5 overflow-y-auto px-5 py-4 text-sm">
      <div class="flex items-center gap-3">
        <div class="text-4xl" aria-hidden="true">🏆</div>
        <div>
          <div class="text-base font-semibold">Chasse aux succès</div>
          <div class="text-xs text-muted">Version {version || "…"} · logiciel libre (GPL-3.0)</div>
        </div>
      </div>

      <!-- Transparence : à garder bien visible -->
      <section class="rounded-xl border border-accent/40 bg-accent/10 p-3">
        <h3 class="font-semibold">Entièrement « vibe-codé » avec Claude</h3>
        <p class="mt-1 text-muted">
          Tout le code de ce logiciel a été écrit par Claude, l'IA d'Anthropic, à partir des idées, des demandes et des
          tests de son auteur, qui n'a pas écrit le code lui-même. Il peut donc contenir des erreurs : n'hésitez pas à les
          signaler.
        </p>
      </section>

      <dl class="space-y-3">
        <div>
          <dt class="text-xs font-semibold tracking-wide text-muted uppercase">Code source</dt>
          <dd class="mt-0.5">
            {#if GITHUB_URL}
              <button onclick={() => open(GITHUB_URL)} class="text-accent underline-offset-2 hover:underline">{GITHUB_URL.replace("https://", "")}</button>
            {:else}
              <span class="text-muted">Dépôt GitHub bientôt disponible.</span>
            {/if}
          </dd>
        </div>
        <div>
          <dt class="text-xs font-semibold tracking-wide text-muted uppercase">Un problème, une idée ?</dt>
          <dd class="mt-0.5">
            Écrivez-moi sur Discord :
            <button
              onclick={copyDiscord}
              title="Copier le pseudo"
              class="rounded bg-raised px-1.5 py-0.5 font-mono font-semibold hover:brightness-125">{DISCORD}</button
            >
            {#if copied}<span class="text-xs text-ok" role="status">copié</span>{/if}
          </dd>
        </div>
      </dl>

      <p class="text-xs text-muted">
        Ce logiciel n'est ni affilié à Valve ni approuvé par Valve. Steam est une marque de Valve Corporation. Il ne modifie
        jamais les fichiers de Steam : il les lit seulement.
      </p>
    </div>
  </div>
</div>
