<script lang="ts">
    import { libraryStore, type Album, type LocalTrack } from "$lib/stores/library.svelte";
    import { audioStore } from "$lib/stores/audio.svelte";
    import { onMount } from "svelte";

    let { album, onBack } = $props<{ album: Album, onBack: () => void }>();
    
    let tracks = $state<LocalTrack[]>([]);
    let artUrl = $state<string | null>(null);

    let activeDropdown = $state<number | null>(null);
    let isCreatingPlaylistForTrack = $state<number | null>(null);
    let newPlaylistName = $state("");

    onMount(() => {
        (async () => {
            tracks = await libraryStore.getAlbumTracks(album.id);
            if (tracks.length > 0) {
                artUrl = await libraryStore.getArtworkUrl(tracks[0].id, tracks[0].file_path);
            }
        })();
        
        const closeDropdowns = () => {
            activeDropdown = null;
            isCreatingPlaylistForTrack = null;
        };
        document.addEventListener('click', closeDropdowns);
        return () => document.removeEventListener('click', closeDropdowns);
    });

    function toggleDropdown(e: Event, index: number) {
        e.stopPropagation();
        if (activeDropdown === index) {
            activeDropdown = null;
            isCreatingPlaylistForTrack = null;
        } else {
            activeDropdown = index;
            isCreatingPlaylistForTrack = null;
        }
    }

    async function addTrackToPlaylist(e: Event, playlistId: number, trackId: number) {
        e.stopPropagation();
        await libraryStore.addToPlaylist(playlistId, trackId);
        activeDropdown = null;
    }

    async function createAndAddPlaylist(e: Event, trackId: number) {
        e.stopPropagation();
        if (!newPlaylistName.trim()) return;
        await libraryStore.createPlaylist(newPlaylistName);
        const newPlaylist = libraryStore.playlists.find(p => p.name === newPlaylistName);
        if (newPlaylist) {
            await libraryStore.addToPlaylist(newPlaylist.id, trackId);
        }
        newPlaylistName = "";
        isCreatingPlaylistForTrack = null;
        activeDropdown = null;
    }

    function playTrack(index: number) {
        audioStore.setQueue(
            tracks.map(t => ({
                id: t.id,
                title: t.title,
                artist: t.artist,
                file_path: t.file_path,
            })),
            index
        );
    }
</script>

<button class="ghost" style="margin-bottom: 2rem; padding: 0.5rem 0;" onclick={onBack}>
    ← Back to Library
</button>

<div class="album-header">
    <div class="art glass-panel">
        {#if artUrl}
            <img src={artUrl} alt={album.title} />
        {:else}
            <div class="placeholder">🎵</div>
        {/if}
    </div>
    <div class="info">
        <h1 style="font-size: 3rem; margin-bottom: 0.5rem;">{album.title}</h1>
        <h2 class="text-muted" style="font-size: 1.5rem;">{album.artist || "Unknown Artist"}</h2>
    </div>
</div>

<div class="track-list" style="margin-top: 3rem;">
    {#each tracks as track, i}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="track-card" class:now-playing={audioStore.currentTrack === track.file_path} onclick={() => playTrack(i)} style="position: relative;">
            <div style="display: flex; gap: 1rem; align-items: center;">
                <span class="text-muted" style="width: 20px; text-align: right;">{track.track_number || i + 1}</span>
                <strong>{track.title}</strong>
            </div>
            <div style="display: flex; gap: 1rem; align-items: center;">
                <span class="text-muted">▶</span>
                <button class="ghost" style="padding: 0 0.5rem;" onclick={(e) => toggleDropdown(e, i)}>⋮</button>
                
                {#if activeDropdown === i}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <div class="dropdown-menu glass-panel" onclick={(e) => e.stopPropagation()}>
                        <div class="dropdown-header">Add to Playlist</div>
                        <div class="dropdown-items">
                            {#each libraryStore.playlists as playlist}
                                <button class="dropdown-item" onclick={(e) => addTrackToPlaylist(e, playlist.id, track.id)}>
                                    {playlist.name}
                                </button>
                            {/each}
                        </div>
                        <div style="padding: 0.5rem; border-top: 1px solid var(--glass-border);">
                            {#if isCreatingPlaylistForTrack === i}
                                <div style="display: flex; gap: 0.5rem;">
                                    <input type="text" bind:value={newPlaylistName} placeholder="Name..." style="width: 100px; padding: 0.2rem;" />
                                    <button class="primary" style="padding: 0.2rem 0.5rem; font-size: 0.8rem;" onclick={(e) => createAndAddPlaylist(e, track.id)}>Add</button>
                                </div>
                            {:else}
                                <button class="ghost dropdown-item" style="color: var(--color-cyan); padding: 0;" onclick={(e) => { e.stopPropagation(); isCreatingPlaylistForTrack = i; }}>
                                    + New Playlist
                                </button>
                            {/if}
                        </div>
                    </div>
                {/if}
            </div>
        </div>
    {/each}
</div>

<style>
    .album-header {
        display: flex;
        gap: 2rem;
        align-items: flex-end;
    }
    .art {
        width: 250px;
        height: 250px;
        flex-shrink: 0;
        display: flex;
        justify-content: center;
        align-items: center;
        background: rgba(0,0,0,0.2);
        overflow: hidden;
    }
    .art img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }
    .placeholder {
        font-size: 5rem;
        opacity: 0.5;
    }
    .dropdown-menu {
        position: absolute;
        right: 2rem;
        top: 2.5rem;
        width: 200px;
        z-index: 10;
        display: flex;
        flex-direction: column;
        box-shadow: 0 10px 30px rgba(0,0,0,0.5);
        border-radius: 8px;
        overflow: hidden;
    }
    .dropdown-header {
        padding: 0.5rem 1rem;
        font-size: 0.8rem;
        color: var(--color-chalk-muted);
        border-bottom: 1px solid var(--glass-border);
    }
    .dropdown-items {
        max-height: 200px;
        overflow-y: auto;
    }
    .dropdown-item {
        width: 100%;
        text-align: left;
        padding: 0.5rem 1rem;
        background: transparent;
        border: none;
        color: white;
        cursor: pointer;
        font-size: 0.9rem;
    }
    .dropdown-item:hover {
        background: rgba(255,255,255,0.1);
    }
</style>
