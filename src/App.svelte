<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let version = $state("");

  invoke<string>("app_version")
    .then((v) => (version = v))
    .catch(() => (version = "hors Tauri"));
</script>

<main>
  <h1>Chasse aux succès</h1>
  <p class="muted">Squelette — étape 1 · v{version}</p>
</main>

<style>
  main {
    height: 100vh;
    display: grid;
    place-content: center;
    text-align: center;
  }
  h1 {
    margin: 0 0 0.5rem;
    font-weight: 600;
  }
  .muted {
    margin: 0;
    opacity: 0.6;
  }
</style>
