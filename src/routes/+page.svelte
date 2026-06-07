<script lang="ts">
    import { onMount } from "svelte";
    import { audioStore } from "$lib/stores/audio.svelte";
    import { libraryStore, type Album } from "$lib/stores/library.svelte";
    
    import Sidebar from "$lib/components/Sidebar.svelte";
    import PlayerBar from "$lib/components/PlayerBar.svelte";
    import SettingsModal from "$lib/components/SettingsModal.svelte";
    import QueueSidebar from "$lib/components/QueueSidebar.svelte";
    import KeyboardHandler from "$lib/components/KeyboardHandler.svelte";
    import ToastContainer from "$lib/components/ToastContainer.svelte";
    
    import AlbumGrid from "$lib/components/AlbumGrid.svelte";
    import AlbumDetail from "$lib/components/AlbumDetail.svelte";
    import PlaylistView from "$lib/components/PlaylistView.svelte";
    import PlaylistDetail from "$lib/components/PlaylistDetail.svelte";
    import type { Playlist } from "$lib/stores/library.svelte";

    let activeView = $state("albums");
    let selectedAlbum = $state<Album | null>(null);
    let selectedPlaylist = $state<Playlist | null>(null);
    let queueOpen = $state(false);

    onMount(() => {
        // Fire-and-forget async init
        (async () => {
            await audioStore.init();
            await libraryStore.fetchAlbums();
            await libraryStore.fetchPlaylists();
        })();

        // Listen for escape events from KeyboardHandler
        const handleEscape = () => {
            if (activeView === "settings") {
                activeView = "albums";
            } else if (selectedAlbum) {
                selectedAlbum = null;
            } else if (selectedPlaylist) {
                selectedPlaylist = null;
            } else if (queueOpen) {
                queueOpen = false;
            }
        };
        document.addEventListener('echo:escape', handleEscape);
        return () => document.removeEventListener('echo:escape', handleEscape);
    });
</script>

<KeyboardHandler />

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
            {#if selectedPlaylist}
                <PlaylistDetail 
                    playlist={selectedPlaylist} 
                    onBack={() => selectedPlaylist = null} 
                    onDeleted={() => selectedPlaylist = null} 
                />
            {:else}
                <PlaylistView onSelectPlaylist={(p) => selectedPlaylist = p} />
            {/if}
        {:else if activeView === "providers"}
            <div style="padding-top: 5rem; text-align: center;">
                <h1 class="text-muted">Network Providers</h1>
                <p>Coming soon...</p>
            </div>
        {/if}
    </main>

    <PlayerBar bind:queueOpen />

    {#if activeView === "settings"}
        <SettingsModal onClose={() => activeView = "albums"} />
    {/if}
</div>

<QueueSidebar bind:open={queueOpen} />
<ToastContainer />
