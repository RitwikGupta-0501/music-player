<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let audioPath = $state("/home/user/Music/test_track.mp3");

  async function loadAndPlay() { await invoke("load_audio", { path: audioPath }); }
  async function play() { await invoke("play_audio"); }
  async function pause() { await invoke("pause_audio"); }
  async function stop() { await invoke("stop_audio"); }
</script>

<main class="container">
  <h1>Echo Core Engine Test</h1>

  <div class="controls">
    <input type="text" bind:value={audioPath} placeholder="Absolute path to .mp3 or .flac" />
    <div class="buttons">
      <button onclick={loadAndPlay}>Load & Play</button>
      <button onclick={play}>Play</button>
      <button onclick={pause}>Pause</button>
      <button onclick={stop}>Stop</button>
    </div>
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    background-color: #1a1a1a;
  }
  .container { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100vh; font-family: system-ui, sans-serif; color: #ffffff; }
  .controls { display: flex; flex-direction: column; gap: 1rem; width: 100%; max-width: 400px; }
  input { padding: 0.8rem; border-radius: 6px; border: 1px solid #333; background: #2a2a2a; color: white; }
  .buttons { display: flex; gap: 0.5rem; justify-content: center; }
  button { padding: 0.6rem 1rem; border: none; border-radius: 6px; background-color: #3b82f6; color: white; cursor: pointer; font-weight: bold; }
  button:hover { background-color: #2563eb; }
</style>
