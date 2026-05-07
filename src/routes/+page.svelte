<script lang="ts">
    import { onMount } from "svelte";
    import { audioStore } from "$lib/stores/audio.svelte";
    import { libraryStore, type Album } from "$lib/stores/library.svelte";
    
    import Sidebar from "$lib/components/Sidebar.svelte";
    import PlayerBar from "$lib/components/PlayerBar.svelte";
    import SettingsModal from "$lib/components/SettingsModal.svelte";
    
    import AlbumGrid from "$lib/components/AlbumGrid.svelte";
    import AlbumDetail from "$lib/components/AlbumDetail.svelte";
    import PlaylistView from "$lib/components/PlaylistView.svelte";

    let activeView = $state("albums");
    let selectedAlbum = $state<Album | null>(null);

    onMount(async () => {
        await audioStore.init();
        await libraryStore.fetchAlbums();
        await libraryStore.fetchPlaylists();
    });
</script>

<div class="app-container">
    <Sidebar bind:activeView />

    <main class="main-content">
        {#if activeView === "albums"}
            {#if selectedAlbum}
                <AlbumDetail album={selectedAlbum} onBack={() => selectedAlbum = null} />
            {:else}
                <AlbumGrid onSelectAlbum={(a) => selectedAlbum = a} />
            {/if}
        {:else if activeView === "playlists"}
            <PlaylistView />
        {:else if activeView === "providers"}
            <div style="padding-top: 5rem; text-align: center;">
                <h1 class="text-muted">Network Providers</h1>
                <p>Coming soon...</p>
            </div>
        {/if}
    </main>

    <PlayerBar />

    {#if activeView === "settings"}
        <SettingsModal onClose={() => activeView = "albums"} />
    {/if}
</div>
