<script lang="ts">
    import { libraryStore, type Playlist } from "$lib/stores/library.svelte";

    let newPlaylistName = $state("");

    async function handleCreate() {
        if (!newPlaylistName.trim()) return;
        await libraryStore.createPlaylist(newPlaylistName);
        newPlaylistName = "";
    }
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

<div class="playlist-grid">
    {#each libraryStore.playlists as playlist}
        <div class="playlist-card glass-panel">
            <div class="icon">📝</div>
            <h3>{playlist.name}</h3>
        </div>
    {/each}
</div>

<style>
    .playlist-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
        gap: 1.5rem;
    }
    .playlist-card {
        padding: 2rem;
        display: flex;
        align-items: center;
        gap: 1.5rem;
        cursor: pointer;
        transition: transform 0.2s, background 0.2s;
    }
    .playlist-card:hover {
        transform: translateY(-2px);
        background: rgba(31, 40, 51, 0.9);
    }
    .icon {
        font-size: 2.5rem;
    }
</style>
