<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { onMount, onDestroy } from "svelte";

    let audioPath = $state("/home/user/Music/test_track.mp3");
    let playbackState = $state("Stopped");
    let currentTrack = $state("None");

    let unlistenState: () => void;
    let unlistenTrack: () => void;

    // Set up event listeners when the component loads
    onMount(async () => {
        unlistenState = await listen<string>("player-state", (event) => {
            playbackState = event.payload;
        });

        unlistenTrack = await listen<string>("current-track", (event) => {
            currentTrack = event.payload;
        });
    });

    // Clean up listeners if the component is destroyed
    onDestroy(() => {
        if (unlistenState) unlistenState();
        if (unlistenTrack) unlistenTrack();
    });

    async function loadAndPlay() {
        await invoke("load_audio", { path: audioPath });
    }
    async function play() {
        await invoke("play_audio");
    }
    async function pause() {
        await invoke("pause_audio");
    }
    async function stop() {
        await invoke("stop_audio");
    }
</script>

<main class="container">
    <h1>Echo Core Engine Test</h1>

    <div class="status-board">
        <p>
            <strong>State:</strong>
            <span class="highlight">{playbackState}</span>
        </p>
        <p>
            <strong>Track:</strong>
            <span class="highlight-path">{currentTrack}</span>
        </p>
    </div>

    <div class="controls">
        <input
            type="text"
            bind:value={audioPath}
            placeholder="Absolute path to .mp3 or .flac"
        />
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
    .container {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        height: 100vh;
        font-family: system-ui, sans-serif;
        color: #ffffff;
    }

    /* New Styles for the Status Board */
    .status-board {
        background: #2a2a2a;
        padding: 1.5rem;
        border-radius: 8px;
        margin-bottom: 2rem;
        width: 100%;
        max-width: 400px;
        border: 1px solid #444;
    }
    .status-board p {
        margin: 0.5rem 0;
    }
    .highlight {
        color: #3b82f6;
        font-weight: bold;
        text-transform: uppercase;
    }
    .highlight-path {
        color: #10b981;
        font-family: monospace;
        font-size: 0.85rem;
        word-break: break-all;
    }

    .controls {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        width: 100%;
        max-width: 400px;
    }
    input {
        padding: 0.8rem;
        border-radius: 6px;
        border: 1px solid #333;
        background: #2a2a2a;
        color: white;
    }
    .buttons {
        display: flex;
        gap: 0.5rem;
        justify-content: center;
    }
    button {
        padding: 0.6rem 1rem;
        border: none;
        border-radius: 6px;
        background-color: #3b82f6;
        color: white;
        cursor: pointer;
        font-weight: bold;
    }
    button:hover {
        background-color: #2563eb;
    }
</style>
