<script lang="ts">
  // Fenêtre compacte : toujours au premier plan, pour un 2ᵉ écran ou par-dessus un jeu en fenêtré.
  // Affiche les cartes épinglées du jeu ouvert dans la fenêtre principale : coche du succès, et leurs
  // compteurs, chronos, vidéos et listes (les « éléments »).
  //
  // Raccourcis globaux (enregistrés par cette fenêtre, seulement tant qu'elle est ouverte) :
  // - Haut / Bas : élément précédent / suivant (la sélection boucle) ;
  // - Gauche / Droite : compteur −1 / +1 ; chrono remis à zéro (à l'arrêt seulement) / lancé ou arrêté ;
  //   vidéo −10 s / lancée, lecture ou pause ; liste : décoche le dernier coché / coche le suivant.
  // Les coches de succès ne sont pas des éléments : pas de succès marqué obtenu d'une fausse touche.
  // Un seul lecteur vidéo ouvert à la fois, pour garder la fenêtre lisible.
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, tick } from "svelte";
  import CompactVideo from "./lib/blocks/CompactVideo.svelte";
  import {
    counterAdd,
    listCheckNext,
    listUncheckLast,
    timerElapsed,
    timerReset,
    timerRunning,
    timerToggle,
  } from "./lib/blocks/ops";
  import { blocks, blocksOf, flushBlocks, listenBlockChanges, loadBlocks, saveBlock } from "./lib/blocks/store.svelte";
  import { formatDuration, kindInfo, type Block, type CounterData, type ListData, type TimerData } from "./lib/blocks/types";
  import { rarityText } from "./lib/filters";
  import { api, cardKey, type AchievementRow, type CardKey, type FreeCard, type GameSummary, type WorkspaceItem } from "./lib/library";
  import { clampOpacity, loadSettings, setCompactOpacity, settings } from "./lib/settings.svelte";
  import {
    applyShortcuts,
    formatAccel,
    loadShortcuts,
    releaseShortcuts,
    shortcuts,
    type ShortcutAction,
  } from "./lib/shortcuts.svelte";
  import { applyMotionAttribute, loadMotion, motion } from "./lib/motion.svelte";
  import { appearance, applyTheme, loadAppearance, type ThemeId } from "./lib/theme.svelte";
  import { beep, chime, loadSound, sound } from "./lib/sound.svelte";
  import { on, send } from "./lib/sync";
  import SteamProgress from "./lib/SteamProgress.svelte";

  let appid = $state(0);
  let game = $state<GameSummary | null>(null);
  let achievements = $state<AchievementRow[]>([]);
  let workspace = $state.raw<WorkspaceItem[]>([]);
  let freeCards = $state.raw<FreeCard[]>([]);
  let now = $state(Date.now());
  let flash = $state<{ text: string; ok: boolean } | null>(null);
  let error = $state("");
  /** Élément sélectionné (id du bloc) ; à défaut, le premier. */
  let selectedId = $state<number | null>(null);
  /** Vidéo dont le lecteur est ouvert. */
  let openVideo = $state<number | null>(null);
  /** Lecteurs des vidéos affichées (pour les actions des raccourcis). */
  const videos: Record<number, ReturnType<typeof CompactVideo>> = {};

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
  const isElement = (b: Block) => b.kind === "counter" || b.kind === "timer" || b.kind === "video" || b.kind === "list";
  const elementsOf = (key: CardKey) => blocksOf(key).filter(isElement);
  /** Éléments dans l'ordre d'affichage : c'est l'ordre de Haut / Bas. */
  const elements = $derived(pinned.flatMap((p) => elementsOf(p.key)));
  const selected = $derived(elements.find((b) => b.id === selectedId) ?? elements[0] ?? null);

  async function load(id: number) {
    if (!id) return;
    if (id !== appid) openVideo = null;
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
    } catch (e) {
      error = String(e);
    }
  }

  /** Enregistre tout de suite (la fenêtre peut être fermée à tout moment) et prévient l'autre fenêtre. */
  async function commit(b: Block) {
    saveBlock(b);
    await flushBlocks();
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
  function listToggle(b: Block, i: number) {
    const item = (b.data as ListData).items[i];
    if (!item) return;
    item.checked = !item.checked;
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

  /** État « raw » : on remplace la carte pour que l'affichage suive. */
  function setFreeDone(id: number, done: boolean) {
    freeCards = freeCards.map((f) => (f.id === id ? { ...f, done } : f));
  }

  async function toggleFree(card: FreeCard) {
    const done = !card.done;
    const set = (d: boolean) => setFreeDone(card.id, d);
    set(done);
    if (done) chime();
    try {
      await api.freeCardSetDone(card.id, done);
      send("free-card-changed", { id: card.id, done });
    } catch (e) {
      set(!done);
      error = String(e);
    }
  }

  // ----- Raccourcis globaux -----

  function feedback(ok: boolean, text: string) {
    flash = { ok, text };
    setTimeout(() => flash?.text === text && (flash = null), 1800);
    if (shortcuts.sound) beep(ok);
  }

  /** Nom de la carte qui porte un bloc. */
  const cardName = (b: Block) =>
    b.achievement_id != null
      ? (byId.get(b.achievement_id)?.name ?? "")
      : freeById.get(b.card_id ?? 0)?.title || "Carte sans titre";
  /** Nom d'un élément dans le retour visuel : son libellé, sinon son type et sa carte. */
  const labelOf = (b: Block) =>
    "label" in b.data && b.data.label ? b.data.label : `${kindInfo(b.kind).label} · ${cardName(b)}`;

  async function select(b: Block) {
    selectedId = b.id;
    await tick();
    document.querySelector(`[data-element="${b.id}"]`)?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }

  function run(action: ShortcutAction) {
    if (!selected) return feedback(false, "Aucun élément : épinglez une carte qui contient un compteur, un chrono, une vidéo ou une liste");
    if (action === "select_prev" || action === "select_next") {
      const i = elements.indexOf(selected);
      const n = elements.length;
      const next = elements[(i + (action === "select_next" ? 1 : -1) + n) % n];
      select(next);
      return feedback(true, labelOf(next));
    }
    const right = action === "act_right";
    const b = selected;
    const name = labelOf(b);
    switch (b.kind) {
      case "counter": {
        const d = b.data as CounterData;
        counter(b, right ? 1 : -1);
        return feedback(true, `${name} : ${d.value}${d.goal != null ? ` / ${d.goal}` : ""}`);
      }
      case "timer": {
        const d = b.data as TimerData;
        if (right) {
          const what = timerToggle(d, Date.now());
          now = Date.now();
          commit(b);
          if (what === "started") return feedback(true, `${name} : lancé`);
          return feedback(true, `${name} : ${formatDuration(d.elapsed_ms)}${what === "record" ? " · nouveau record !" : ""}`);
        }
        if (timerRunning(d)) return feedback(false, `${name} : arrêtez-le d'abord (Droite) pour le remettre à zéro`);
        timer(b, true);
        return feedback(true, `${name} : remis à zéro`);
      }
      case "list": {
        const d = b.data as ListData;
        const text = right ? listCheckNext(d) : listUncheckLast(d);
        if (text == null) return feedback(false, right ? "Liste terminée" : "Rien à décocher");
        commit(b);
        const done = d.items.filter((x) => x.checked).length;
        return feedback(true, `${right ? "✓" : "✗"} ${text || "élément"} (${done} / ${d.items.length})`);
      }
      case "video": {
        const r = right ? videos[b.id]?.right() : videos[b.id]?.left();
        if (r) feedback(r.ok, r.text);
        return;
      }
    }
  }

  const registerShortcuts = () => loadShortcuts().then(() => applyShortcuts(run));
  // Interface simulée : pas de raccourcis globaux dans un navigateur, on les déclenche depuis la console
  if (import.meta.env.DEV) (window as unknown as { compactRun: typeof run }).compactRun = run;

  async function close() {
    await releaseShortcuts();
    await flushBlocks();
    try {
      await getCurrentWindow().close();
    } catch {
      window.close();
    }
  }

  /** Un chrono affiché tourne-t-il ? (lancé ici ou dans la fenêtre principale) */
  const anyTimerRunning = $derived(elements.some((b) => b.kind === "timer" && timerRunning(b.data as TimerData)));
  // Horloge des chronos : seulement quand l'un d'eux tourne, la fenêtre reste ouverte pendant le jeu
  $effect(() => {
    if (!anyTimerRunning) return;
    now = Date.now();
    const id = setInterval(() => (now = Date.now()), 100);
    return () => clearInterval(id);
  });

  $effect(applyMotionAttribute);
  $effect(applyTheme);

  onMount(() => {
    document.documentElement.classList.add("compact");
    const stops = [
      listenBlockChanges(),
      on("compact-game", ({ appid: id }) => load(id)),
      on("data-changed", ({ appid: id }) => id === appid && load(id)),
      on("achievement-changed", ({ id, unlocked, game: g }) => {
        const a = achievements.find((x) => x.id === id);
        if (a) a.unlocked = unlocked;
        if (g.appid === appid) game = g;
      }),
      on("free-card-changed", ({ id, done }) => setFreeDone(id, done)),
      on("shortcuts-changed", registerShortcuts),
      // Succès débloqué en jeu (actualisation automatique)
      on("steam-progress", async ({ game: g, unlocked }) => {
        if (g.appid !== appid) {
          if (unlocked.length) flash = { ok: true, text: `🏆 ${g.name} : ${unlocked.map((u) => u.name).join(", ")}` };
        } else {
          game = g;
          const ach = await api.achievements(appid).catch(() => null);
          if (ach && g.appid === appid) achievements = ach;
          if (unlocked.length) flash = { ok: true, text: `🏆 ${unlocked.map((u) => u.name).join(", ")}` };
        }
        if (unlocked.length) {
          chime();
          const text = flash?.text;
          // Plus long qu'un retour de raccourci : le joueur a les yeux sur le jeu
          setTimeout(() => flash?.text === text && (flash = null), 10000);
        }
      }),
      on("compact-opacity", ({ value }) => (settings.compactOpacity = clampOpacity(value))),
      on("motion-changed", ({ pref }) => (motion.pref = pref)),
      on("theme-changed", ({ mode, dark, light }) => Object.assign(appearance, { mode, dark: dark as ThemeId, light: light as ThemeId })),
      on("sound-changed", ({ check }) => (sound.check = check)),
      on("data-restored", () => location.reload()),
    ];
    // Fermeture par Alt+F4 ou par le système : les touches sont rendues aux autres logiciels
    let stopClose: (() => void) | null = null;
    try {
      getCurrentWindow()
        .onCloseRequested(() => releaseShortcuts())
        .then((u) => (stopClose = u));
    } catch {
      /* interface simulée */
    }

    Promise.all([loadSettings(), loadMotion(), loadSound(), loadAppearance(), registerShortcuts()]).then(async () => {
      send("compact-ready", {});
      // Sans réponse de la fenêtre principale (interface simulée) : premier jeu de la bibliothèque
      setTimeout(async () => {
        if (!appid) load((await api.list().catch(() => []))[0]?.appid ?? 0);
      }, 400);
    });
    return () => {
      stops.forEach((s) => s());
      stopClose?.();
    };
  });

  /** Couleur du thème rendue transparente selon le réglage d'opacité (fonds seulement). */
  const seeThrough = (color: string, factor = 1) =>
    `color-mix(in srgb, var(${color}) ${Math.round(settings.compactOpacity * factor * 100)}%, transparent)`;

  const spoiler = (a: AchievementRow) => settings.antiSpoiler && a.hidden && !a.unlocked;

  /** Rappel des touches : « Ctrl + Alt + ↑ ↓ : choisir · ← → : agir » quand elles ont les mêmes modificateurs. */
  const keysHint = $derived.by(() => {
    const k = shortcuts.keys;
    const all = [k.select_prev, k.select_next, k.act_left, k.act_right];
    if (all.every((x) => x)) {
      const mods = (x: string) => x.split("+").slice(0, -1).join("+");
      const key = (x: string) => formatAccel(x.split("+").at(-1) ?? x);
      const m = mods(all[0]!);
      if (all.every((x) => mods(x!) === m)) {
        const prefix = m ? `${formatAccel(m)} + ` : "";
        return `${prefix}${key(all[0]!)} ${key(all[1]!)} : choisir · ${key(all[2]!)} ${key(all[3]!)} : agir`;
      }
    }
    return `${formatAccel(k.select_prev)} / ${formatAccel(k.select_next)} : choisir · ${formatAccel(k.act_left)} / ${formatAccel(k.act_right)} : agir`;
  });
</script>

{#snippet element(b: Block)}
  {@const isSelected = selected?.id === b.id}
  <div
    data-element={b.id}
    role="presentation"
    onpointerdown={() => (selectedId = b.id)}
    class="rounded-lg px-2 py-1.5 transition {isSelected ? 'bg-accent/10 ring-2 ring-accent' : 'ring-1 ring-transparent'}"
    style:background-color={isSelected ? undefined : seeThrough("--color-bg", 0.6)}
  >
    {#if b.kind === "counter"}
      {@const d = b.data as CounterData}
      {@const reached = d.goal != null && d.value >= d.goal}
      <div class="flex items-center gap-2">
        <div class="min-w-0 flex-1">
          <div class="truncate text-[11px] text-muted"># {d.label || "Compteur"}</div>
          <div class="font-mono text-lg leading-tight font-bold tabular-nums {reached ? 'text-ok' : ''}">
            {d.value}{#if d.goal != null}<span class="text-sm font-normal text-muted"> / {d.goal}</span>{/if}
          </div>
        </div>
        <button onclick={() => counter(b, -1)} disabled={d.value <= 0} class="h-8 w-8 rounded-md border border-line font-semibold hover:bg-raised disabled:opacity-40" aria-label="Retirer">−</button>
        <button onclick={() => counter(b, 1)} class="h-8 w-8 rounded-md bg-accent font-semibold text-on-accent hover:brightness-110" aria-label="Ajouter">+</button>
      </div>
    {:else if b.kind === "timer"}
      {@const d = b.data as TimerData}
      {@const running = timerRunning(d)}
      {@const elapsed = timerElapsed(d, now)}
      <div class="flex items-center gap-2">
        <div class="min-w-0 flex-1">
          <div class="truncate text-[11px] text-muted">⏱ {d.label || "Chrono"}</div>
          <div class="font-mono text-lg leading-tight font-bold tabular-nums {d.target_ms != null && elapsed > d.target_ms ? 'text-danger' : ''}">
            {formatDuration(elapsed)}
            {#if d.best_ms != null}<span class="text-xs font-normal text-muted">· meilleur {formatDuration(d.best_ms)}</span>{/if}
          </div>
        </div>
        <button
          onclick={() => timer(b)}
          class="h-8 rounded-md px-2 text-sm font-semibold hover:brightness-110 {running ? 'bg-danger text-white' : 'bg-accent text-on-accent'}"
        >
          {running ? "Stop" : "Go"}
        </button>
        <button onclick={() => timer(b, true)} disabled={running || elapsed === 0} class="h-8 w-8 rounded-md border border-line text-sm text-muted hover:bg-raised disabled:opacity-40" title="Remettre à zéro (à l'arrêt)">0</button>
      </div>
    {:else if b.kind === "list"}
      {@const d = b.data as ListData}
      {@const done = d.items.filter((x) => x.checked).length}
      <div class="flex items-center gap-2">
        <div class="min-w-0 flex-1">
          <div class="truncate text-[11px] text-muted">☑ Liste</div>
          <div class="font-mono text-lg leading-tight font-bold tabular-nums {d.items.length && done === d.items.length ? 'text-ok' : ''}">
            {done}<span class="text-sm font-normal text-muted"> / {d.items.length}</span>
          </div>
        </div>
      </div>
      {#if d.items.length}
        <ul class="mt-1 max-h-32 space-y-0.5 overflow-y-auto pr-1">
          {#each d.items as item, i (i)}
            <li>
              <label class="flex cursor-pointer items-start gap-1.5 text-xs">
                <input type="checkbox" checked={item.checked} onchange={() => listToggle(b, i)} class="mt-0.5 accent-accent" />
                <span class={item.checked ? "text-muted line-through" : ""}>{item.text || "—"}</span>
              </label>
            </li>
          {/each}
        </ul>
      {/if}
    {:else if b.kind === "video"}
      <CompactVideo
        bind:this={videos[b.id]}
        block={b as Block<"video">}
        open={openVideo === b.id}
        onopen={() => {
          openVideo = b.id;
          selectedId = b.id;
        }}
        onclose={() => (openVideo = null)}
      />
    {/if}
  </div>
{/snippet}

<!-- L'opacité réglable ne touche que les fonds : texte, icônes et vidéo restent nets par-dessus un jeu -->
<div
  class="flex h-full flex-col overflow-hidden rounded-xl border border-line text-fg"
  style:background-color={seeThrough("--color-bg")}
>
  <!-- Barre de titre (déplace la fenêtre) -->
  <header
    data-tauri-drag-region
    class="flex shrink-0 items-center gap-2 border-b border-line px-2.5 py-1.5 select-none"
    style:background-color={seeThrough("--color-panel")}
  >
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
      title="Opacité du fond"
      aria-label="Opacité du fond de la fenêtre"
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

    <!-- Cartes épinglées -->
    {#each pinned as { key, a, free } (key)}
      <section class="space-y-1.5">
        {#if free}
          <div class="flex items-center gap-2">
            <button
              onclick={() => toggleFree(free)}
              aria-pressed={free.done}
              aria-label={free.done ? "Marquer comme à faire" : "Marquer comme faite"}
              class="grid h-5 w-5 shrink-0 place-items-center rounded-full border-2 text-[10px] transition
                {free.done ? 'border-ok bg-ok text-bg' : 'border-line hover:border-muted'}"
            >
              {#if free.done}✓{/if}
            </button>
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
              {#if a.progress && !a.unlocked && !spoiler(a)}
                <SteamProgress progress={a.progress} small />
              {:else if a.rarity != null}
                <div class="text-[10px] {rarityText(a.rarity)}">{a.rarity.toFixed(1)} %</div>
              {/if}
            </div>
          </div>
        {/if}
        {#each elementsOf(key) as b (b.id)}
          {@render element(b)}
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

  {#if elements.length}
    <!-- Rappel des raccourcis -->
    <footer class="shrink-0 border-t border-line px-2.5 py-1 text-[10px] text-muted" title="Réglables dans les Réglages de la fenêtre principale">
      {keysHint}
    </footer>
  {/if}
</div>
