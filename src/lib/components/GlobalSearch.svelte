<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { audioStore } from "$lib/stores/audio.svelte";
    import { toastStore } from "$lib/stores/toast.svelte";

    let { isOpen = $bindable(false) } = $props();

    let query = $state("");
    let localResults = $state<any[]>([]);
    let isSearching = $state(false);
    let alternatives = $state<Record<number, any[]>>({});

    let inputRef = $state<HTMLInputElement | null>(null);

    $effect(() => {
        if (isOpen && inputRef) {
            inputRef.focus();
        }
    });

    let searchTimeout: ReturnType<typeof setTimeout>;

    function handleInput() {
        if (searchTimeout) clearTimeout(searchTimeout);
        
        if (!query.trim()) {
            localResults = [];
            alternatives = {};
            return;
        }

        searchTimeout = setTimeout(async () => {
            isSearching = true;
            try {
                // 1. Instant Local Search
                localResults = await invoke("search_library", { query, limit: 10 });
                
                // 2. Background Async Gathering
                const providers: any[] = await invoke("get_providers");
                const enabledProviders = providers.filter(p => p.status === 'enabled' && p.capabilities?.includes('search'));
                
                const searchPromises = enabledProviders.map(p => 
                    invoke("search_provider", { providerId: p.id, query })
                        .then((results: any) => (results || []).map((r: any) => ({ ...r, provider_name: p.name, provider_id: p.id })))
                        .catch(e => {
                            console.error(`Search failed for provider ${p.name}:`, e);
                            return [];
                        })
                );
                
                const resultsArray = await Promise.all(searchPromises);
                const remoteResults: any[] = resultsArray.flat();

                // 3. Fuzzy Matching
                if (remoteResults && remoteResults.length > 0) {
                    for (const local of localResults) {
                        const matches: any[] = await invoke("fuzzy_match_tracks", {
                            localTrack: local,
                            remoteTracks: remoteResults
                        });
                        if (matches.length > 0) {
                            alternatives[local.id] = matches;
                        }
                    }
                }
            } catch (e) {
                console.error("Search failed", e);
            } finally {
                isSearching = false;
            }
        }, 300);
    }

    function playTrack(track: any) {
        if (audioStore.queue.length > 0) {
            if (audioStore.trackClickBehavior === "interrupt") {
                audioStore.playInterrupt(track);
                isOpen = false;
                return;
            } else if (audioStore.trackClickBehavior === "append") {
                audioStore.addToQueue(track);
                toastStore.show("Added to queue", 'info', 1500);
                isOpen = false;
                return;
            }
        }
        audioStore.setQueue([track], 0);
        isOpen = false;
    }
    
    function playAlternative(alt: any) {
        const payload = {
            title: alt.title,
            artist: alt.artist,
            stream_url: alt.stream_url,
            provider_id: alt.provider_id ?? 'remote',
            quality_hint: alt.quality_hint ?? null,
            cover_art_url: alt.cover_art_url ?? null,
        };
        
        if (audioStore.queue.length > 0) {
            if (audioStore.trackClickBehavior === "interrupt") {
                audioStore.playInterrupt(payload);
                isOpen = false;
                return;
            } else if (audioStore.trackClickBehavior === "append") {
                audioStore.addToQueue(payload);
                toastStore.show("Added to queue", 'info', 1500);
                isOpen = false;
                return;
            }
        }
        
        audioStore.setQueue([payload], 0);
        isOpen = false;
    }

    let openPopoverId = $state<number | null>(null);
</script>

{#if isOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal-backdrop" onclick={(e) => { if (e.target === e.currentTarget) isOpen = false; }}>
        <div class="search-modal">
            <div class="search-header">
                <input 
                    bind:this={inputRef}
                    bind:value={query}
                    oninput={handleInput}
                    placeholder="Search music..."
                    class="search-input"
                />
                {#if isSearching}
                    <div class="spinner"></div>
                {/if}
            </div>

            <div class="results-list">
                {#each localResults as track (track.id)}
                    <div class="track-row">
                        <div class="track-info" onclick={() => playTrack(track)}>
                            <div class="track-title">{track.title}</div>
                            <div class="track-artist">{track.artist || 'Unknown Artist'}</div>
                        </div>
                        
                        {#if alternatives[track.id]}
                            <div class="alternatives-wrapper">
                                <button class="alt-btn" onclick={() => openPopoverId = openPopoverId === track.id ? null : track.id}>
                                    Sources ▾
                                </button>
                                
                                {#if openPopoverId === track.id}
                                    <div class="popover">
                                        <div class="popover-title">Alternative Sources</div>
                                        {#each alternatives[track.id] as alt}
                                            <button class="popover-item" onclick={() => playAlternative(alt)}>
                                                {alt.title} ({alt.provider_name || 'Remote'})
                                            </button>
                                        {/each}
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    </div>
                {/each}
                {#if query && localResults.length === 0 && !isSearching}
                    <div class="no-results">No local results found</div>
                {/if}
            </div>
        </div>
    </div>
{/if}

<style>
    .modal-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        backdrop-filter: blur(4px);
        z-index: 1000;
        display: flex;
        align-items: flex-start;
        justify-content: center;
        padding-top: 10vh;
    }

    .search-modal {
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: 12px;
        width: 100%;
        max-width: 600px;
        box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
        overflow: visible;
        display: flex;
        flex-direction: column;
    }

    .search-header {
        padding: 16px;
        border-bottom: 1px solid var(--border);
        display: flex;
        align-items: center;
        gap: 12px;
    }

    .search-input {
        flex: 1;
        background: transparent;
        border: none;
        color: var(--text-primary);
        font-size: 1.2rem;
        outline: none;
    }

    .search-input::placeholder {
        color: var(--text-muted);
    }

    .spinner {
        width: 20px;
        height: 20px;
        border: 2px solid var(--border);
        border-top-color: var(--primary);
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }

    .results-list {
        max-height: 400px;
        overflow-y: auto;
        padding: 8px;
    }

    .track-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 12px;
        border-radius: 8px;
        cursor: pointer;
        position: relative;
    }

    .track-row:hover {
        background: var(--surface-hover);
    }

    .track-info {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .track-title {
        color: var(--text-primary);
        font-weight: 500;
    }

    .track-artist {
        color: var(--text-muted);
        font-size: 0.9rem;
    }

    .alternatives-wrapper {
        position: relative;
    }

    .alt-btn {
        background: var(--surface-light);
        border: 1px solid var(--border);
        color: var(--text-secondary);
        padding: 4px 8px;
        border-radius: 6px;
        font-size: 0.8rem;
        cursor: pointer;
    }

    .alt-btn:hover {
        background: var(--surface-hover);
        color: var(--text-primary);
    }

    .popover {
        position: absolute;
        top: 100%;
        right: 0;
        margin-top: 8px;
        background: var(--surface-light);
        border: 1px solid var(--border);
        border-radius: 8px;
        padding: 8px;
        min-width: 200px;
        box-shadow: 0 10px 20px rgba(0, 0, 0, 0.3);
        z-index: 1010;
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .popover-title {
        font-size: 0.75rem;
        color: var(--text-muted);
        text-transform: uppercase;
        letter-spacing: 0.5px;
        padding: 4px 8px;
        border-bottom: 1px solid var(--border);
        margin-bottom: 4px;
    }

    .popover-item {
        background: transparent;
        border: none;
        color: var(--text-primary);
        text-align: left;
        padding: 8px;
        border-radius: 4px;
        cursor: pointer;
        font-size: 0.9rem;
    }

    .popover-item:hover {
        background: var(--surface-hover);
    }

    .no-results {
        padding: 24px;
        text-align: center;
        color: var(--text-muted);
    }
</style>
