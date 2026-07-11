<script lang="ts">
    import { libraryStore, type Playlist } from "$lib/stores/library.svelte";
    import { flip } from "svelte/animate";
    import { cubicOut } from "svelte/easing";

    let { onSelectPlaylist } = $props<{ onSelectPlaylist: (p: Playlist) => void }>();

    let newPlaylistName = $state("");
    let mosaics = $state<Record<number, string[]>>({});

    async function handleCreate() {
        if (!newPlaylistName.trim()) return;
        await libraryStore.createPlaylist(newPlaylistName);
        newPlaylistName = "";
        await loadMosaics();
    }

    async function loadMosaics() {
        const entries = await Promise.all(
            libraryStore.playlists.map(async p => [p.id, await libraryStore.getPlaylistArtworkMosaic(p.id)] as const)
        );
        mosaics = Object.fromEntries(entries);
    }

    // Reactively load mosaics when playlists change
    $effect(() => {
        if (libraryStore.playlists) {
            loadMosaics();
        }
    });
</script>

<div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 2rem;">
    <div>
        <h2 class="text-cyan" style="font-size: 2.5rem; margin-bottom: 0.5rem;">Playlists</h2>
        <span class="text-muted">{libraryStore.playlists.length} Playlists</span>
    </div>
    
    <div style="display: flex; gap: 0.5rem;">
        <input
            type="text"
            bind:value={newPlaylistName}
            placeholder="New playlist name..."
            spellcheck="false"
            style="width: 250px;"
        />
        <button class="primary" onclick={handleCreate}>
            Create
        </button>
    </div>
</div>

{#if libraryStore.playlists.length === 0}
    <div class="empty-state">
        <p class="text-muted" style="font-size: 1.1rem; margin-bottom: 1.5rem;">No playlists yet. Create your first one!</p>
        <div style="display: flex; gap: 0.5rem;">
            <input
                type="text"
                bind:value={newPlaylistName}
                placeholder="My Playlist"
                spellcheck="false"
                style="width: 250px;"
            />
            <button class="primary" onclick={handleCreate} disabled={!newPlaylistName.trim()}>
                Create
            </button>
        </div>
    </div>
{:else}
    <div class="playlist-grid">
        {#each libraryStore.playlists as playlist (playlist.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div 
                class="playlist-card glass-panel" 
                onclick={() => onSelectPlaylist(playlist)}
                style:view-transition-name="playlist-{playlist.id}"
            >
                <div class="art">
                    {#if mosaics[playlist.id] && mosaics[playlist.id].length >= 4}
                        <div class="mosaic">
                            <img src={mosaics[playlist.id][0]} alt="Cover" />
                            <img src={mosaics[playlist.id][1]} alt="Cover" />
                            <img src={mosaics[playlist.id][2]} alt="Cover" />
                            <img src={mosaics[playlist.id][3]} alt="Cover" />
                        </div>
                    {:else if mosaics[playlist.id] && mosaics[playlist.id].length > 0}
                        <img src={mosaics[playlist.id][0]} alt="Cover" style="width: 100%; height: 100%; object-fit: cover;" />
                    {:else}
                        <div class="icon">📝</div>
                    {/if}
                </div>
                <h3 style="margin-top: 0.5rem; text-align: center; width: 100%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">{playlist.name}</h3>
            </div>
        {/each}
    </div>
{/if}

<style>
    .playlist-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
        gap: 2rem;
        padding: 2.5rem;
        padding-bottom: 10rem;
    }
    .playlist-card {
        display: flex;
        flex-direction: column;
        cursor: pointer;
        transition: transform 0.22s cubic-bezier(0.34, 1.56, 0.64, 1);
    }
    .playlist-card:hover {
        transform: translateY(-4px);
    }
    .art {
        width: 100%;
        aspect-ratio: 1;
        background-color: #27272a;
        border-radius: 1.5rem;
        overflow: hidden;
        display: flex;
        justify-content: center;
        align-items: center;
        border: 1px solid rgba(255, 255, 255, 0.1);
        box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.5);
    }
    .mosaic {
        display: grid;
        grid-template-columns: 1fr 1fr;
        grid-template-rows: 1fr 1fr;
        width: 100%;
        height: 100%;
    }
    .mosaic img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }
    .icon {
        font-size: 3rem;
        opacity: 0.5;
    }
    .empty-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        height: 50vh;
    }
</style>
