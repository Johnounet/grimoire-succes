<script lang="ts">
  // Écran de prototype de l'étape 2 : affiche en texte brut ce que renvoie chaque source Steam.
  import { onMount } from "svelte";
  import {
    steamDetect,
    steamProbe,
    type DetectReport,
    type GameAchievements,
    type Outcome,
  } from "./lib/steam";

  let detect = $state<DetectReport | null>(null);
  let appid = $state("620");
  let lang = $state("french");
  let profile = $state("");
  let output = $state("");
  let busy = $state(false);

  onMount(async () => {
    try {
      detect = await steamDetect();
    } catch (e) {
      output = `Détection impossible : ${e}`;
    }
  });

  const date = (t: number | null) => (t ? new Date(t * 1000).toLocaleString("fr-FR") : "");

  function section(label: string, o: Outcome<GameAchievements>): string {
    if ("err" in o) return `=== ${label} ===\nÉCHEC : ${o.err}\n`;
    const g = o.ok;
    const list = g.achievements;
    const done = list.filter((a) => a.unlocked).length;
    const ids = list.filter((a) => a.api_name).length;
    const lines = list.map((a) => {
      const state = a.unlocked === true ? "[x]" : a.unlocked === false ? "[ ]" : "[?]";
      const rarity = a.rarity != null ? `${a.rarity.toFixed(1)} %` : "";
      const flags = [a.hidden ? "caché" : "", a.dlc_appid ? `DLC ${a.dlc_appid}` : ""].filter(Boolean).join(", ");
      return (
        `${state} ${(a.api_name ?? "—").padEnd(32)} ${a.name}` +
        `${flags ? `  (${flags})` : ""}\n` +
        `      ${a.description}\n` +
        `      rareté ${rarity.padEnd(8)} ${date(a.unlock_time)}`
      );
    });
    return (
      `=== ${label} === ${g.game_name ?? ""} — ${list.length} succès, ${done} obtenus, ` +
      `${ids} avec ID technique, langue ${g.language}\n` +
      lines.join("\n") +
      "\n"
    );
  }

  async function run(e: Event) {
    e.preventDefault();
    const id = Number(appid.trim());
    if (!Number.isInteger(id) || id <= 0) {
      output = "AppID invalide";
      return;
    }
    busy = true;
    output = "Interrogation des sources…";
    try {
      const r = await steamProbe(id, lang, profile);
      output = [
        section("CACHE LOCAL", r.local_cache),
        section("PROFIL PUBLIC", r.public_profile),
        section("PAGE PUBLIQUE", r.public_page),
        `API rareté : ${"ok" in r.rarity_api_count ? `${r.rarity_api_count.ok} succès` : r.rarity_api_count.err}`,
      ].join("\n");
    } catch (err) {
      output = `Erreur : ${err}`;
    } finally {
      busy = false;
    }
  }
</script>

<main>
  <h1>Prototype des sources Steam</h1>

  <pre class="info">{#if detect}Steam : {detect.steam_path ?? "introuvable"}
Comptes : {detect.accounts
        .map((a) => `${a.persona_name ?? "?"} (${a.account_id})${a.most_recent ? " ★" : ""}`)
        .join(", ") || "aucun"}
Jeux en cache local : {detect.cached_games.length}{:else}Détection de Steam…{/if}</pre>

  <form onsubmit={run}>
    <label>AppID <input bind:value={appid} size="10" /></label>
    <label>
      Langue
      <select bind:value={lang}>
        <option value="french">français</option>
        <option value="english">anglais</option>
      </select>
    </label>
    <label class="grow">
      Profil (facultatif)
      <input bind:value={profile} placeholder="lien de profil Steam ou SteamID64 — sinon compte détecté" />
    </label>
    <button disabled={busy}>Interroger</button>
  </form>

  <pre class="out">{output}</pre>
</main>

<style>
  main {
    height: 100vh;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
  }
  h1 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
  }
  form {
    display: flex;
    gap: 0.75rem;
    align-items: end;
    flex-wrap: wrap;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.85rem;
    opacity: 0.9;
  }
  .grow {
    flex: 1;
    min-width: 16rem;
  }
  input,
  select,
  button {
    font: inherit;
    padding: 0.4rem 0.6rem;
    border-radius: 6px;
    border: 1px solid #343a48;
    background: #1b1e27;
    color: inherit;
  }
  button {
    background: #3b6ef5;
    border-color: #3b6ef5;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
  }
  pre {
    margin: 0;
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 0.8rem;
    white-space: pre-wrap;
  }
  .info {
    opacity: 0.7;
  }
  .out {
    flex: 1;
    overflow: auto;
    padding: 0.75rem;
    border-radius: 8px;
    background: #0c0e12;
  }
</style>
