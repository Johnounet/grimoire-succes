<script lang="ts">
  // Réglages : interface (animations, son), étiquettes, raccourcis clavier globaux, mode compact.
  import { backdrop, motion, pop, setMotionPref, type MotionPref } from "./motion.svelte";
  import { chime, setCheckSound, sound } from "./sound.svelte";
  import { setCompactOpacity, settings } from "./settings.svelte";
  import {
    accelFromEvent,
    DEFAULT_SHORTCUTS,
    formatAccel,
    setShortcut,
    setShortcutSound,
    shortcutActions,
    shortcuts,
    type ShortcutAction,
  } from "./shortcuts.svelte";
  import { createTag, deleteTag, TAG_COLORS, tags, tagStyle, updateTag, type Tag } from "./tags.svelte";

  let { onclose }: { onclose: () => void } = $props();

  // ----- Interface -----
  const motionChoices: { v: MotionPref; l: string }[] = [
    { v: "system", l: "Selon Windows" },
    { v: "on", l: "Activées" },
    { v: "off", l: "Désactivées" },
  ];

  // ----- Étiquettes -----
  let tagError = $state("");
  let newName = $state("");
  let newColor = $state(TAG_COLORS[5]);
  let confirmDelete = $state<number | null>(null);
  let colorFor = $state<number | null>(null);

  async function add(e: SubmitEvent) {
    e.preventDefault();
    const r = await createTag(newName, newColor);
    if (typeof r === "string") tagError = r;
    else {
      tagError = "";
      newName = "";
    }
  }

  async function rename(t: Tag, name: string) {
    if (name.trim() === t.name) return;
    tagError = (await updateTag(t.id, name, t.color)) ?? "";
  }

  async function recolor(t: Tag, color: string) {
    colorFor = null;
    tagError = (await updateTag(t.id, t.name, color)) ?? "";
  }

  async function remove(t: Tag) {
    confirmDelete = null;
    tagError = (await deleteTag(t.id)) ?? "";
  }

  // ----- Raccourcis -----
  let capturing = $state<ShortcutAction | null>(null);

  function capture(e: KeyboardEvent, action: ShortcutAction) {
    if (capturing !== action) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      capturing = null;
      return;
    }
    const accel = accelFromEvent(e);
    if (!accel) return; // on attend la touche après les modificateurs
    capturing = null;
    setShortcut(action, accel);
  }
</script>

<div
  class="fixed inset-0 z-50 grid place-items-center bg-black/60 p-4"
  role="presentation"
  transition:backdrop
  onclick={(e) => e.target === e.currentTarget && onclose()}
  onkeydown={(e) => e.key === "Escape" && !capturing && onclose()}
>
  <div
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title" transition:pop
    class="flex max-h-[90vh] w-full max-w-2xl flex-col rounded-2xl border border-line bg-panel shadow-2xl"
  >
    <header class="flex items-center border-b border-line px-5 py-3">
      <h2 id="settings-title" class="flex-1 text-lg font-semibold">Réglages</h2>
      <button onclick={onclose} class="rounded px-2 py-1 text-muted hover:bg-raised hover:text-fg" aria-label="Fermer">✕</button>
    </header>

    <div class="space-y-8 overflow-y-auto px-5 py-4">
      <!-- Interface -->
      <section aria-labelledby="s-ui">
        <h3 id="s-ui" class="text-sm font-semibold tracking-wide text-muted uppercase">Interface</h3>
        <div class="mt-3 flex flex-wrap items-center gap-3 text-sm">
          <span id="motion-label">Animations</span>
          <div class="inline-flex rounded-lg border border-line bg-bg p-0.5" role="radiogroup" aria-labelledby="motion-label">
            {#each motionChoices as o (o.v)}
              <button
                role="radio"
                aria-checked={motion.pref === o.v}
                onclick={() => setMotionPref(o.v)}
                class="rounded-md px-3 py-1 transition
                  {motion.pref === o.v ? 'bg-raised font-medium text-fg' : 'text-muted hover:text-fg'}"
              >
                {o.l}
              </button>
            {/each}
          </div>
        </div>
        <p class="mt-1 text-xs text-muted">
          Coche, apparitions, déplacements des cartes, confettis du 100 %.
          {#if motion.pref === "system"}
            Windows demande en ce moment {motion.system ? "de réduire les animations : elles sont coupées" : "des animations normales"}.
          {/if}
        </p>
        <label class="mt-3 flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={sound.check}
            onchange={(e) => {
              setCheckSound(e.currentTarget.checked);
              chime(); // aperçu du son
            }}
            class="accent-accent"
          />
          Petit carillon quand je coche un succès
        </label>
      </section>

      <!-- Étiquettes -->
      <section aria-labelledby="s-tags">
        <h3 id="s-tags" class="text-sm font-semibold tracking-wide text-muted uppercase">Étiquettes</h3>
        <p class="mt-1 text-xs text-muted">Communes à tous les jeux. Supprimer une étiquette la retire de tous les succès.</p>
        <ul class="mt-3 space-y-1.5">
          {#each tags.list as t (t.id)}
            <li class="relative flex items-center gap-2">
              <button
                onclick={() => (colorFor = colorFor === t.id ? null : t.id)}
                class="h-6 w-6 shrink-0 rounded-full border-2 border-bg ring-1 ring-line"
                style="background: {t.color}"
                aria-label={`Couleur de « ${t.name} »`}
                title="Changer la couleur"
              ></button>
              <input
                value={t.name}
                onchange={(e) => rename(t, e.currentTarget.value)}
                aria-label="Nom de l'étiquette"
                class="min-w-0 flex-1 rounded-md border border-transparent bg-transparent px-2 py-1 text-sm outline-none hover:border-line focus:border-accent"
              />
              <span class="rounded-full border px-1.5 py-px text-[10px] leading-4 font-medium" style={tagStyle(t.color)}>{t.name}</span>
              {#if confirmDelete === t.id}
                <button onclick={() => remove(t)} class="rounded-md bg-danger px-2 py-1 text-xs font-medium text-white">Supprimer</button>
                <button onclick={() => (confirmDelete = null)} class="rounded-md px-2 py-1 text-xs text-muted hover:text-fg">Annuler</button>
              {:else}
                <button
                  onclick={() => (confirmDelete = t.id)}
                  class="rounded px-2 py-1 text-muted hover:bg-raised hover:text-danger"
                  aria-label={`Supprimer « ${t.name} »`}
                  title="Supprimer">🗑</button
                >
              {/if}
              {#if colorFor === t.id}
                <div class="absolute top-full left-0 z-10 mt-1 flex gap-1.5 rounded-lg border border-line bg-raised p-2 shadow-xl">
                  {#each TAG_COLORS as c (c)}
                    <button
                      onclick={() => recolor(t, c)}
                      class="h-6 w-6 rounded-full {c === t.color ? 'ring-2 ring-fg' : ''}"
                      style="background: {c}"
                      aria-label={`Couleur ${c}`}
                    ></button>
                  {/each}
                </div>
              {/if}
            </li>
          {/each}
        </ul>
        <form onsubmit={add} class="mt-3 flex items-center gap-2">
          <div class="flex gap-1">
            {#each TAG_COLORS as c (c)}
              <button
                type="button"
                onclick={() => (newColor = c)}
                class="h-5 w-5 rounded-full {c === newColor ? 'ring-2 ring-fg ring-offset-2 ring-offset-panel' : ''}"
                style="background: {c}"
                aria-label={`Couleur ${c}`}
                aria-pressed={c === newColor}
              ></button>
            {/each}
          </div>
          <input
            bind:value={newName}
            placeholder="Nouvelle étiquette"
            aria-label="Nom de la nouvelle étiquette"
            class="min-w-0 flex-1 rounded-md border border-line bg-bg px-2 py-1 text-sm outline-none focus:border-accent"
          />
          <button type="submit" disabled={!newName.trim()} class="rounded-md bg-accent px-3 py-1 text-sm font-medium text-white disabled:opacity-50">
            Créer
          </button>
        </form>
        {#if tagError}<p class="mt-2 text-sm text-danger">{tagError}</p>{/if}
      </section>

      <!-- Raccourcis -->
      <section aria-labelledby="s-keys">
        <h3 id="s-keys" class="text-sm font-semibold tracking-wide text-muted uppercase">Raccourcis clavier globaux</h3>
        <p class="mt-1 text-xs text-muted">
          Ils marchent même quand le jeu est au premier plan, sur le compteur et le chrono marqués ◎ actifs dans leurs blocs.
          Sous Linux Wayland, ils peuvent ne pas fonctionner selon le bureau.
        </p>
        <table class="mt-3 w-full text-sm">
          <tbody>
            {#each shortcutActions as s (s.action)}
              <tr class="border-b border-line/50 last:border-0">
                <td class="py-2 pr-3">{s.label}</td>
                <td class="py-2">
                  <button
                    onclick={() => (capturing = capturing === s.action ? null : s.action)}
                    onkeydown={(e) => capture(e, s.action)}
                    onblur={() => capturing === s.action && (capturing = null)}
                    aria-label={`Changer le raccourci : ${s.label}`}
                    class="min-w-40 rounded-md border px-2 py-1 font-mono text-xs
                      {capturing === s.action ? 'border-accent bg-accent/10 text-fg' : 'border-line bg-bg hover:border-muted'}"
                  >
                    {capturing === s.action ? "Appuyez sur la combinaison…" : formatAccel(shortcuts.keys[s.action])}
                  </button>
                  {#if shortcuts.errors[s.action]}<p class="mt-1 text-xs text-danger">{shortcuts.errors[s.action]}</p>{/if}
                </td>
                <td class="py-2 text-right text-xs whitespace-nowrap">
                  {#if shortcuts.keys[s.action] !== DEFAULT_SHORTCUTS[s.action]}
                    <button onclick={() => setShortcut(s.action, DEFAULT_SHORTCUTS[s.action])} class="text-muted underline hover:text-fg">
                      par défaut
                    </button>
                  {/if}
                  {#if shortcuts.keys[s.action]}
                    <button onclick={() => setShortcut(s.action, null)} class="ml-2 text-muted underline hover:text-fg">aucun</button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        <label class="mt-3 flex items-center gap-2 text-sm">
          <input type="checkbox" checked={shortcuts.sound} onchange={(e) => setShortcutSound(e.currentTarget.checked)} class="accent-accent" />
          Petit son de confirmation
        </label>
      </section>

      <!-- Mode compact -->
      <section aria-labelledby="s-compact">
        <h3 id="s-compact" class="text-sm font-semibold tracking-wide text-muted uppercase">Mode compact</h3>
        <p class="mt-1 text-xs text-muted">
          Petite fenêtre toujours au premier plan : cartes épinglées (icône punaise), leurs compteurs et chronos, et les blocs actifs.
        </p>
        <label class="mt-3 flex items-center gap-3 text-sm">
          Opacité
          <input
            type="range"
            min="0.3"
            max="1"
            step="0.05"
            value={settings.compactOpacity}
            oninput={(e) => setCompactOpacity(Number(e.currentTarget.value))}
            class="flex-1 accent-accent"
          />
          <span class="w-10 text-right tabular-nums">{Math.round(settings.compactOpacity * 100)} %</span>
        </label>
      </section>
    </div>
  </div>
</div>
