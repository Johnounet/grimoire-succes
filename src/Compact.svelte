<script lang="ts">
  // Fenêtre compacte : toujours au premier plan, pour un 2ᵉ écran ou par-dessus un jeu en fenêtré.
  // Affiche les cartes épinglées du jeu ouvert dans la fenêtre principale (coche, compteurs, chronos)
  // et, en tête, le compteur et le chrono actifs des raccourcis clavier (qui peuvent être d'un autre jeu).
  // Pas de vidéo ici : la fenêtre reste légère (question ouverte du SPEC, tranchée ainsi pour la V1).
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { counterAdd, timerElapsed, timerReset, timerRunning, timerToggle } from "./lib/blocks/ops";
  import { blocks, blocksOf, findBlock, flushBlocks, listenBlockChanges, loadBlocks, saveBlock } from "./lib/blocks/store.svelte";
  import { formatDuration, type Block, type CounterData, type TimerData } from "./lib/blocks/types";
  import { rarityText } from "./lib/filters";
  import { api, cardKey, type AchievementRow, type FreeCard, type GameSummary, type WorkspaceItem } from "./lib/library";
  import { clampOpacity, loadSettings, setCompactOpacity, settings } from "./lib/settings.svelte";
  import { active, formatAccel, loadActive, shortcutContext, shortcuts, target, toggleActive } from "./lib/shortcuts.svelte";
  import { applyMotionAttribute, loadMotion, motion } from "./lib/motion.svelte";
  import { chime, loadSound, sound } from "./lib/sound.svelte";
  import { on, send } from "./lib/sync";

  let appid = $state(0);
  let game = $state<GameSummary | null>(null);
  let achievements = $state<AchievementRow[]>([]);
  let workspace = $state.raw<WorkspaceItem[]>([]);
  let freeCards = $state.raw<FreeCard[]>([]);
  /** Blocs actifs d'un autre jeu (ceux du jeu ouvert sont dans le store). */
  let outside = $state<Record<number, { block: Block; name: string }>>({});
  let now = $state(Date.now());
  let flash = $state<{ text: string; ok: boolean } | null>(null);
  let error = $state("");

  const byId = $derived(new Map(achievements.map((a) => [a.id, a])));
  const freeById = $derived(new Map(freeCards.map((f) => [f.id, f])));
  /** Cartes épinglées : succès ou cartes libres. */
  const pinned = $derived(
    workspace.flatMap((it) => {
      if (!it.pinned) return [];
      const a = it.achievement_id != null ? byId.get(it.achievement_id) : undefined;
      const free = it.card_id != null ? freeById.get(it.card_id) : undefined;
      return a || free ? [{ key: cardKey(it), a, free }] : [];
    }),
  );
  /** Nom de la carte qui porte un bloc du jeu ouvert. */
  const cardName = (b: Block) =>
    b.achievement_id != null ? (byId.get(b.achievement_id)?.name ?? "") : (freeById.get(b.card_id ?? 0)?.title || "Carte sans titre");
  const tracked = (b: Block) => b.kind === "counter" || b.kind === "timer";

  async function load(id: number) {
    if (!id) return;
    appid = id;
    error = "";
    try {
      const [games, ach, ws, free] = await Promise.all([
        api.list(),
        api.achievements(id),
        api.workspaceList(id),
        api.freeCards(id),
        loadBlocks(id),
      ]);
      if (appid !== id) return;
      game = games.find((g) => g.appid === id) ?? null;
      achievements = ach;
      workspace = ws;
      freeCards = free;
      shortcutContext.workspace = ws;
      await loadOutside();
    } catch (e) {
      error = String(e);
    }
  }

  async function loadOutside() {
    const next: typeof outside = {};
    for (const id of [active.counter, active.timer]) {
      if (id == null || findBlock(id)) continue;
      const info = await invoke<{ block: Block; card_title: string } | null>("block_get", { id }).catch(() => null);
      if (info) next[id] = { block: info.block, name: info.card_title };
    }
    outside = next;
  }

  /** Bloc actif à afficher en tête : celui du store s'il est chargé, sinon la copie lue en base. */
  function activeBlock(id: number | null): { block: Block; name: string } | null {
    if (id == null) return null;
    const b = findBlock(id);
    if (b) return { block: b, name: cardName(b) };
    return outside[id] ?? null;
  }

  /** Enregistre tout de suite (la fenêtre peut être fermée à tout moment) et prévient l'autre fenêtre. */
  async function commit(b: Block) {
    if (findBlock(b.id)) {
      saveBlock(b);
      await flushBlocks();
    } else {
      await invoke("block_update", { id: b.id, data: $state.snapshot(b.data) }).catch((e) => (error = String(e)));
      send("block-changed", { block: $state.snapshot(b) as Block });
    }
  }

  function counter(b: Block, sign: 1 | -1) {
    counterAdd(b.data as CounterData, sign);
    commit(b);
  }
  function timer(b: Block, reset = false) {
    if (reset) timerReset(b.data as TimerData);
    else timerToggle(b.data as TimerData, Date.now());
    now = Date.now();
    commit(b);
  }

  async function toggle(a: AchievementRow) {
    const unlocked = !a.unlocked;
    a.unlocked = unlocked;
    if (unlocked) chime();
    try {
      const g = await api.setUnlocked(a.id, unlocked);
      game = g;
      send("achievement-changed", { id: a.id, unlocked, game: g });
    } catch (e) {
      a.unlocked = !unlocked;
      error = String(e);
    }
  }

  async function close() {
    await flushBlocks();
    try {
      await getCurrentWindow().close();
    } catch {
      window.close();
    }
  }

  $effect(applyMotionAttribute);

  onMount(() => {
    document.documentElement.classList.add("compact");
    const stops = [
      listenBlockChanges(),
      on("block-changed", ({ block }) => {
        if (outside[block.id]) outside[block.id].block.data = block.data;
      }),
      on("compact-game", ({ appid: id }) => load(id)),
      on("data-changed", ({ appid: id }) => id === appid && load(id)),
      on("achievement-changed", ({ id, unlocked, game: g }) => {
        const a = achievements.find((x) => x.id === id);
        if (a) a.unlocked = unlocked;
        if (g.appid === appid) game = g;
      }),
      on("active-changed", ({ counter, timer }) => {
        active.counter = counter;
        active.timer = timer;
        loadOutside();
      }),
      on("compact-opacity", ({ value }) => (settings.compactOpacity = clampOpacity(value))),
      on("motion-changed", ({ pref }) => (motion.pref = pref)),
      on("sound-changed", ({ check }) => (sound.check = check)),
      on("shortcut-fired", (f) => {
        flash = f;
        setTimeout(() => flash?.text === f.text && (flash = null), 1800);
      }),
    ];
    const tick = setInterval(() => (now = Date.now()), 100);

    Promise.all([loadSettings(), loadActive(), loadMotion(), loadSound()]).then(async () => {
      send("compact-ready", {});
      // Sans réponse de la fenêtre principale (interface simulée) : premier jeu de la bibliothèque
      setTimeout(async () => {
        if (!appid) load((await api.list().catch(() => []))[0]?.appid ?? 0);
      }, 400);
    });
    return () => {
      stops.forEach((s) => s());
      clearInterval(tick);
    };
  });

  const spoiler = (a: AchievementRow) => settings.antiSpoiler && a.hidden && !a.unlocked;
</script>

{#snippet activeToggle(b: Block)}
  {@const kind = b.kind as "counter" | "timer"}
  {@const t = target(kind)}
  {@const on = t?.id === b.id}
  {@const keys =
    kind === "counter"
      ? `${formatAccel(shortcuts.keys.counter_inc)} / ${formatAccel(shortcuts.keys.counter_dec)}`
      : `${formatAccel(shortcuts.keys.timer_toggle)}`}
  <button
    onclick={() => toggleActive(kind, b.id)}
    aria-pressed={on && t?.chosen}
    title={on && t?.chosen
      ? `Actif pour les raccourcis clavier (${keys}). Cliquer pour retirer.`
      : on
        ? `Visé par défaut par les raccourcis (${keys}). Cliquer pour le choisir.`
        : `Rendre actif pour les raccourcis clavier (${keys})`}
    class="rounded px-0.5 {on ? 'text-accent' : 'text-muted/50 hover:text-fg'}">◎</button
  >
{/snippet}

{#snippet blockRow(b: Block, title: string, isActive: boolean)}
  {#if b.kind === "counter"}
    {@const d = b.data as CounterData}
    {@const reached = d.goal != null && d.value >= d.goal}
    <div class="flex items-center gap-2 rounded-lg bg-bg/60 px-2 py-1.5 {isActive ? 'ring-1 ring-accent/60' : ''}">
      <div class="min-w-0 flex-1">
        <div class="truncate text-[11px] text-muted">{@render activeToggle(b)} {d.label || title || "Compteur"}</div>
        <div class="font-mono text-lg leading-tight font-bold tabular-nums {reached ? 'text-ok' : ''}">
          {d.value}{#if d.goal != null}<span class="text-sm font-normal text-muted"> / {d.goal}</span>{/if}
        </div>
      </div>
      <button onclick={() => counter(b, -1)} disabled={d.value <= 0} class="h-8 w-8 rounded-md border border-line font-semibold hover:bg-raised disabled:opacity-40" aria-label="Retirer">−</button>
      <button onclick={() => counter(b, 1)} class="h-8 w-8 rounded-md bg-accent font-semibold text-white hover:brightness-110" aria-label="Ajouter">+</button>
    </div>
  {:else if b.kind === "timer"}
    {@const d = b.data as TimerData}
    {@const running = timerRunning(d)}
    {@const elapsed = timerElapsed(d, now)}
    <div class="flex items-center gap-2 rounded-lg bg-bg/60 px-2 py-1.5 {isActive ? 'ring-1 ring-accent/60' : ''}">
      <div class="min-w-0 flex-1">
        <div class="truncate text-[11px] text-muted">{@render activeToggle(b)} {d.label || title || "Chrono"}</div>
        <div class="font-mono text-lg leading-tight font-bold tabular-nums {d.target_ms != null && elapsed > d.target_ms ? 'text-danger' : ''}">
          {formatDuration(elapsed)}
          {#if d.best_ms != null}<span class="text-xs font-normal text-muted">· meilleur {formatDuration(d.best_ms)}</span>{/if}
        </div>
      </div>
      <button
        onclick={() => timer(b)}
        class="h-8 rounded-md px-2 text-sm font-semibold text-white hover:brightness-110 {running ? 'bg-danger' : 'bg-accent'}"
      >
        {running ? "Stop" : "Go"}
      </button>
      <button onclick={() => timer(b, true)} disabled={elapsed === 0 && !running} class="h-8 w-8 rounded-md border border-line text-sm text-muted hover:bg-raised disabled:opacity-40" title="Remettre à zéro">0</button>
    </div>
  {/if}
{/snippet}

<div class="flex h-full flex-col overflow-hidden rounded-xl border border-line bg-bg text-fg" style:opacity={settings.compactOpacity}>
  <!-- Barre de titre (déplace la fenêtre) -->
  <header data-tauri-drag-region class="flex shrink-0 items-center gap-2 border-b border-line bg-panel px-2.5 py-1.5 select-none">
    <span data-tauri-drag-region class="min-w-0 flex-1 truncate text-sm font-semibold">
      {game?.name ?? "Mode compact"}
      {#if game}<span class="font-normal text-muted"> · {game.unlocked}/{game.total}</span>{/if}
    </span>
    <input
      type="range"
      min="0.3"
      max="1"
      step="0.05"
      value={settings.compactOpacity}
      oninput={(e) => setCompactOpacity(Number(e.currentTarget.value))}
      title="Opacité"
      aria-label="Opacité de la fenêtre"
      class="w-16 accent-accent"
    />
    <button onclick={close} class="rounded px-1.5 text-muted hover:bg-raised hover:text-fg" aria-label="Fermer le mode compact" title="Fermer">✕</button>
  </header>

  {#if flash}
    <div class="shrink-0 px-2.5 py-1 text-xs font-medium {flash.ok ? 'bg-accent/20 text-fg' : 'bg-danger/20 text-danger'}" role="status">
      {flash.text}
    </div>
  {/if}

  <div class="min-h-0 flex-1 space-y-3 overflow-y-auto p-2.5">
    {#if error}<p class="text-xs text-danger">{error}</p>{/if}
    {#if blocks.error}<p class="text-xs text-danger">{blocks.error}</p>{/if}

    <!-- Blocs actifs (raccourcis clavier) -->
    {#if activeBlock(active.counter) || activeBlock(active.timer)}
      <section class="space-y-1.5" aria-label="Blocs actifs">
        {#each [activeBlock(active.counter), activeBlock(active.timer)] as x, i (i)}
          {#if x}{@render blockRow(x.block, x.name, true)}{/if}
        {/each}
      </section>
    {/if}

    <!-- Cartes épinglées -->
    {#each pinned as { key, a, free } (key)}
      <section class="space-y-1.5">
        {#if free}
          <div class="flex items-center gap-2">
            <span class="grid h-5 w-5 shrink-0 place-items-center text-xs text-muted" aria-hidden="true">✎</span>
            <div class="min-w-0 flex-1 truncate text-sm font-medium {free.title ? '' : 'text-muted'}">
              {free.title || "Carte sans titre"}
            </div>
          </div>
        {:else if a}
        {@const icon = a.unlocked ? a.icon_url : (a.icon_gray_url ?? a.icon_url)}
        <div class="flex items-center gap-2">
          <button
            onclick={() => toggle(a)}
            aria-pressed={a.unlocked}
            aria-label={a.unlocked ? "Marquer comme non obtenu" : "Marquer comme obtenu"}
            class="grid h-5 w-5 shrink-0 place-items-center rounded-full border-2 text-[10px] transition
              {a.unlocked ? 'border-ok bg-ok text-bg' : 'border-line hover:border-muted'}"
          >
            {#if a.unlocked}✓{/if}
          </button>
          {#if icon}<img src={icon} alt="" class="h-7 w-7 shrink-0 rounded {a.unlocked ? '' : 'opacity-70 grayscale'}" />{/if}
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-medium {spoiler(a) ? 'blur-[5px]' : ''}">{a.name}</div>
            {#if a.rarity != null}<div class="text-[10px] {rarityText(a.rarity)}">{a.rarity.toFixed(1)} %</div>{/if}
          </div>
        </div>
        {/if}
        {#each blocksOf(key).filter(tracked) as b (b.id)}
          {#if b.id !== active.counter && b.id !== active.timer}{@render blockRow(b, "", false)}{/if}
        {/each}
      </section>
    {:else}
      {#if appid}
        <p class="px-2 py-6 text-center text-xs text-muted">
          Épinglez des cartes (icône punaise) dans l'espace de travail pour les suivre ici.
        </p>
      {/if}
    {/each}
  </div>
</div>
