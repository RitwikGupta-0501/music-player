<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { onMount, onDestroy } from "svelte";

    // --- AUDIO DAEMON STATE ---
    let playbackState = $state("Stopped");
    let currentTrack = $state("None");
    let unlistenState: () => void;
    let unlistenTrack: () => void;

    // --- LOCAL LIBRARY STATE ---
    let scanDirectory = $state("");
    let localTracks = $state<Array<{ id: number; title: string; file_path: string }>>([]);
    let isScanning = $state(false);

    // --- SETTINGS STATE ---
    let showSettings = $state(false);
    let defaultScanDir = $state("");

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
        
        // Load settings
        try {
            const savedDir = await invoke<string | null>("get_setting", { key: "default_scan_dir" });
            if (savedDir) {
                defaultScanDir = savedDir;
                scanDirectory = savedDir;
            }
        } catch (e) {
            console.error("Failed to load settings:", e);
        }
    });

    onDestroy(() => {
        if (unlistenState) unlistenState();
        if (unlistenTrack) unlistenTrack();
    });

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

    async function clearLibrary() {
        try {
            await invoke("clear_local_library");
            localTracks = [];
            console.log("Library cleared.");
        } catch (e) {
            console.error("Failed to clear library:", e);
        }
    }

    async function saveSettings() {
        try {
            await invoke("set_setting", { key: "default_scan_dir", value: defaultScanDir });
            showSettings = false;
        } catch (e) {
            console.error("Failed to save settings:", e);
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

<div class="app-container">
    <header class="header" style="display: flex; justify-content: space-between; align-items: center; padding: 2rem;">
        <h1 style="margin: 0; text-align: center; flex: 1;">Echo <span class="text-cyan">Engine</span></h1>
        <button class="primary" style="padding: 0.5rem 1rem;" onclick={() => showSettings = true}>⚙ Settings</button>
    </header>

    <main class="main-content">
        <!-- LOCAL LIBRARY SECTION -->
        <section class="glass-panel" style="padding: 1.5rem;">
            <h2 style="margin-bottom: 1.5rem;">Local Library</h2>
            <div style="display: flex; gap: 0.5rem; margin-bottom: 1.5rem;">
                <input
                    type="text"
                    bind:value={scanDirectory}
                    placeholder="Scan path (e.g., /home/user/Music)"
                    disabled={isScanning}
                />
                <button class="primary" onclick={scanLocalLibrary} disabled={isScanning}>
                    {isScanning ? "..." : "Scan"}
                </button>
            </div>

            {#if localTracks.length > 0}
                <div class="track-list">
                    {#each localTracks as track}
                        <div class="track-card">
                            <div style="display: flex; flex-direction: column; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; padding-right: 1rem;">
                                <strong>{track.title}</strong>
                                <span class="text-muted" style="overflow: hidden; text-overflow: ellipsis;">{track.file_path}</span>
                            </div>
                            <button onclick={() => playSelected(track.file_path)}>Play</button>
                        </div>
                    {/each}
                </div>
            {/if}
        </section>

        <!-- LUA PROVIDER SECTION -->
        <section class="glass-panel" style="padding: 1.5rem;">
            <h2 style="margin-bottom: 1.5rem;">Network Providers</h2>
            <div style="display: flex; gap: 0.5rem; margin-bottom: 1.5rem;">
                <input
                    type="text"
                    bind:value={searchQuery}
                    placeholder="Search query..."
                />
                <button class="primary" onclick={testSearch}>Search</button>
            </div>

            {#if searchResults.length > 0}
                <div class="track-list">
                    {#each searchResults as track}
                        <div class="track-card">
                            <div style="display: flex; flex-direction: column; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; padding-right: 1rem;">
                                <strong>{track.title}</strong>
                                <span class="text-muted">{track.artist}</span>
                            </div>
                            <button onclick={() => playSelected(track.stream_url)}>Play</button>
                        </div>
                    {/each}
                </div>
            {/if}
        </section>
    </main>

    <!-- GLASSMORPHIC BOTTOM PLAYER -->
    <div class="glass-panel bottom-player">
        <div class="player-info">
            <h3 class="text-cyan">{playbackState}</h3>
            <span class="text-muted" style="max-width: 400px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                {currentTrack !== "None" ? currentTrack : "Ready to play"}
            </span>
        </div>
        <div class="player-controls">
            <button onclick={play}>Play</button>
            <button onclick={pause}>Pause</button>
            <button onclick={stop} style="border-color: var(--color-danger); color: var(--color-danger);">Stop</button>
        </div>
    </div>

    <!-- SETTINGS MODAL -->
    {#if showSettings}
        <div style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0,0,0,0.7); display: flex; justify-content: center; align-items: center; z-index: 100; backdrop-filter: blur(4px);">
            <div class="glass-panel" style="padding: 2rem; width: 100%; max-width: 500px; display: flex; flex-direction: column; gap: 1.5rem;">
                <div style="display: flex; justify-content: space-between; align-items: center;">
                    <h2>Settings</h2>
                    <button style="padding: 0.2rem 0.5rem;" onclick={() => showSettings = false}>✕</button>
                </div>
                
                <div>
                    <label style="display: block; margin-bottom: 0.5rem; color: var(--color-chalk-muted);">Default Scan Directory</label>
                    <input type="text" bind:value={defaultScanDir} placeholder="/home/user/Music" />
                </div>

                <div style="border-top: 1px solid var(--glass-border); padding-top: 1.5rem;">
                    <label style="display: block; margin-bottom: 0.5rem; color: var(--color-chalk-muted);">Data Management</label>
                    <button style="border-color: var(--color-danger); color: var(--color-danger); width: 100%;" onclick={clearLibrary}>
                        Wipe Local Database
                    </button>
                </div>

                <div style="display: flex; justify-content: flex-end; gap: 1rem; margin-top: 1rem;">
                    <button onclick={() => showSettings = false}>Cancel</button>
                    <button class="primary" onclick={saveSettings}>Save Changes</button>
                </div>
            </div>
        </div>
    {/if}
</div>
