<script lang="ts">
  // Étiquettes d'un succès : pastilles affichées, et menu pour en poser, en retirer ou en créer.
  import { pop } from "./motion.svelte";
  import { popoverStyle } from "./popover";
  import { createTag, hasTag, TAG_COLORS, tags, tagsOf, tagStyle, toggleTag } from "./tags.svelte";

  let { achievementId, compact = false }: { achievementId: number; compact?: boolean } = $props();

  let open = $state(false);
  let button: HTMLButtonElement | undefined = $state();
  let draft = $state("");
  let error = $state("");
  const mine = $derived(tagsOf(achievementId));

  async function quickCreate(e: SubmitEvent) {
    e.preventDefault();
    const name = draft.trim();
    if (!name) return;
    // Couleur suivante dans la palette, pour varier
    const color = TAG_COLORS[tags.list.length % TAG_COLORS.length];
    const t = await createTag(name, color);
    if (typeof t === "string") {
      error = t;
      return;
    }
    error = "";
    draft = "";
    await toggleTag(achievementId, t.id);
  }
</script>

<div
  class="relative flex flex-wrap items-center gap-1"
  onfocusout={(e) => {
    if (!e.currentTarget.contains(e.relatedTarget as Node | null)) open = false;
  }}
>
  {#each mine as t (t.id)}
    <span class="rounded-full border px-1.5 py-px text-[10px] leading-4 font-medium" style={tagStyle(t.color)}>{t.name}</span>
  {/each}
  <button
    bind:this={button}
    onclick={() => (open = !open)}
    aria-expanded={open}
    aria-haspopup="true"
    title="Étiquettes"
    aria-label="Modifier les étiquettes"
    class="rounded-full border border-dashed border-line px-1.5 py-px text-[10px] leading-4 text-muted transition hover:border-muted hover:text-fg
      {compact && mine.length === 0 ? 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100' : ''}"
  >
    {mine.length ? "±" : "+ étiquette"}
  </button>

  {#if open}
    <div
      role="dialog"
      aria-label="Étiquettes"
      tabindex="-1"
      transition:pop={{ y: -4 }}
      onkeydown={(e) => {
        if (e.key === "Escape") open = false;
      }}
      style={popoverStyle(button, 224)}
      class="z-50 rounded-lg border border-line bg-raised p-1.5 shadow-xl"
    >
      <ul class="max-h-56 overflow-y-auto">
        {#each tags.list as t, i (t.id)}
          <li>
            <label class="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-panel">
              <input
                type="checkbox"
                checked={hasTag(achievementId, t.id)}
                onchange={() => toggleTag(achievementId, t.id)}
                class="accent-accent"
                {@attach (el) => {
                  if (i === 0) el.focus();
                }}
              />
              <span class="h-2.5 w-2.5 shrink-0 rounded-full" style="background: {t.color}"></span>
              <span class="truncate">{t.name}</span>
            </label>
          </li>
        {:else}
          <li class="px-2 py-1 text-xs text-muted">Aucune étiquette pour l'instant.</li>
        {/each}
      </ul>
      <form onsubmit={quickCreate} class="mt-1 border-t border-line pt-1.5">
        <input
          bind:value={draft}
          placeholder="Nouvelle étiquette…"
          aria-label="Nom de la nouvelle étiquette"
          class="w-full rounded-md border border-line bg-bg px-2 py-1 text-sm outline-none focus:border-accent"
        />
        {#if error}<p class="mt-1 text-xs text-danger">{error}</p>{/if}
      </form>
      <p class="mt-1 px-1 text-[10px] text-muted">Couleurs, noms et suppression : Réglages.</p>
    </div>
  {/if}
</div>
