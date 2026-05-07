<script lang="ts">
    import { libraryStore, type Album } from "$lib/stores/library.svelte";
    import AlbumCard from "./AlbumCard.svelte";

    let { onSelectAlbum } = $props<{ onSelectAlbum: (a: Album) => void }>();
    
    let scanDirectory = $state("");
</script>

<div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 2rem;">
    <div>
        <h2 class="text-cyan" style="font-size: 2.5rem; margin-bottom: 0.5rem;">Library</h2>
        <span class="text-muted">{libraryStore.albums.length} Albums</span>
    </div>
    
    <div style="display: flex; gap: 0.5rem;">
        <input
            type="text"
            bind:value={scanDirectory}
            placeholder="Scan path (e.g., /home/user/Music)"
            disabled={libraryStore.isScanning}
            spellcheck="false"
            style="width: 300px;"
        />
        <button class="primary" onclick={() => libraryStore.scanDirectory(scanDirectory)} disabled={libraryStore.isScanning}>
            {libraryStore.isScanning ? "..." : "Scan"}
        </button>
    </div>
</div>

<div class="album-grid">
    {#each libraryStore.albums as album}
        <AlbumCard {album} onclick={() => onSelectAlbum(album)} />
    {/each}
</div>

<style>
    .album-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        gap: 1.5rem;
    }
</style>
