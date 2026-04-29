<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { onMount, onDestroy } from "svelte";

    // --- AUDIO DAEMON STATE ---
    let playbackState = $state("Stopped");
    let currentTrack = $state("None");
    let unlistenState: () => void;
    let unlistenTrack: () => void;

    onMount(async () => {
        unlistenState = await listen<string>(
            "player-state",
            (e) => (playbackState = e.payload),
        );
        unlistenTrack = await listen<string>(
            "current-track",
            (e) => (currentTrack = e.payload),
        );
        await loadLocalTracks();
    });

    onDestroy(() => {
        if (unlistenState) unlistenState();
        if (unlistenTrack) unlistenTrack();
    });

    // --- LOCAL LIBRARY STATE ---
    let scanDirectory = $state("");
    let localTracks = $state<Array<{ id: number; title: string; file_path: string }>>([]);
    let isScanning = $state(false);

    async function scanLocalLibrary() {
        if (!scanDirectory) return;
        isScanning = true;
        try {
            const added = await invoke("scan_local_directory", { path: scanDirectory });
            console.log(`Scanned and added ${added} tracks`);
            await loadLocalTracks();
        } catch (e) {
            console.error("Scan Error:", e);
        } finally {
            isScanning = false;
        }
    }

    async function loadLocalTracks() {
        try {
            localTracks = await invoke("get_local_tracks");
        } catch (e) {
            console.error("Fetch Local Tracks Error:", e);
        }
    }

    // --- LUA PROVIDER STATE ---
    let searchQuery = $state("");
    let searchResults = $state<
        Array<{ id: string; title: string; artist: string; stream_url: string }>
    >([]);

    async function testSearch() {
        try {
            searchResults = await invoke("search_provider", {
                query: searchQuery,
            });
            // Svelte 5 clean logging:
            console.log($state.snapshot(searchResults));
        } catch (e) {
            console.error("Bridge Error:", e);
        }
    }

    // --- PLAYBACK COMMANDS ---
    async function playSelected(stream_url: string) {
        await invoke("load_audio", { path: stream_url });
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
    <h1>Echo Engine</h1>

    <div class="status-board">
        <p>
            <strong>State:</strong>
            <span class="highlight">{playbackState}</span>
        </p>
        <p>
            <strong>Track:</strong>
            <span class="highlight-path">{currentTrack}</span>
        </p>
        <div class="buttons" style="margin-top: 1rem;">
            <button onclick={play}>Play</button>
            <button onclick={pause}>Pause</button>
            <button onclick={stop}>Stop</button>
        </div>
    </div>

    <!-- LOCAL LIBRARY UI -->
    <div class="search-box">
        <input
            type="text"
            bind:value={scanDirectory}
            placeholder="Scan Local Directory (e.g., /home/user/Music)"
            disabled={isScanning}
        />
        <button onclick={scanLocalLibrary} disabled={isScanning}>
            {isScanning ? "Scanning..." : "Scan"}
        </button>
    </div>

    {#if localTracks.length > 0}
        <div class="results" style="margin-bottom: 2rem;">
            <h2>Local Tracks</h2>
            {#each localTracks as track}
                <div class="track-card">
                    <div class="track-info">
                        <strong>{track.title}</strong>
                        <span>{track.file_path}</span>
                    </div>
                    <button
                        class="play-btn"
                        onclick={() => playSelected(track.file_path)}
                        >Play</button
                    >
                </div>
            {/each}
        </div>
    {/if}

    <!-- LUA PROVIDER UI -->
    <div class="search-box">
        <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search via Lua Provider..."
        />
        <button onclick={testSearch}>Search</button>
    </div>

    {#if searchResults.length > 0}
        <div class="results">
            {#each searchResults as track}
                <div class="track-card">
                    <div class="track-info">
                        <strong>{track.title}</strong>
                        <span>{track.artist}</span>
                    </div>
                    <button
                        class="play-btn"
                        onclick={() => playSelected(track.stream_url)}
                        >Play</button
                    >
                </div>
            {/each}
        </div>
    {/if}
</main>

<style>
    :global(body) {
        margin: 0;
        background-color: #1a1a1a;
        font-family: system-ui, sans-serif;
        color: #ffffff;
    }
    .container {
        display: flex;
        flex-direction: column;
        align-items: center;
        padding-top: 2rem;
        height: 100vh;
    }

    .status-board {
        background: #2a2a2a;
        padding: 1.5rem;
        border-radius: 8px;
        margin-bottom: 2rem;
        width: 100%;
        max-width: 400px;
        border: 1px solid #444;
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

    .search-box {
        display: flex;
        gap: 0.5rem;
        width: 100%;
        max-width: 400px;
        margin-bottom: 1.5rem;
    }
    input {
        flex: 1;
        padding: 0.8rem;
        border-radius: 6px;
        border: 1px solid #333;
        background: #2a2a2a;
        color: white;
    }

    .results {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        width: 100%;
        max-width: 400px;
    }
    .results h2 {
        font-size: 1.2rem;
        margin-bottom: 0.5rem;
        color: #ddd;
    }
    .track-card {
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: #222;
        padding: 1rem;
        border-radius: 6px;
        border: 1px solid #333;
    }
    .track-info {
        display: flex;
        flex-direction: column;
    }
    .track-info span {
        font-size: 0.85rem;
        color: #aaa;
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
    button:hover:not(:disabled) {
        background-color: #2563eb;
    }
    button:disabled {
        background-color: #555;
        color: #999;
        cursor: not-allowed;
    }
    .play-btn {
        background-color: #10b981;
    }
    .play-btn:hover:not(:disabled) {
        background-color: #059669;
    }
</style>
