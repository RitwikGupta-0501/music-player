<script lang="ts">
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

    import RightDrawer from "$lib/components/RightDrawer.svelte";
    import FullScreenPlayer from "$lib/components/FullScreenPlayer.svelte";
    import { transitionLayout } from "$lib/utils/transitions";

    let activeView = $state("albums");
    let selectedAlbum = $state<Album | null>(null);
    let selectedPlaylist = $state<Playlist | null>(null);
    let queueOpen = $state(false);
    let fullScreenOpen = $state(false);

    let drawerOpen = $derived(queueOpen || selectedAlbum !== null || selectedPlaylist !== null);
    
    function getDrawerTitle() {
        if (queueOpen) return "Up Next";
        if (selectedAlbum) return "Album Details";
        if (selectedPlaylist) return "Playlist Details";
        return "";
    }

    function closeDrawer() {
        transitionLayout(() => {
            queueOpen = false;
            selectedAlbum = null;
            selectedPlaylist = null;
        });
    }

    $effect(() => {
        (async () => {
            await audioStore.init();
            await libraryStore.fetchAlbums();
            await libraryStore.fetchPlaylists();
        })();

        const handleEscape = () => {
            if (activeView === "settings") {
                activeView = "albums";
            } else {
                closeDrawer();
            }
        };
        document.addEventListener('echo:escape', handleEscape);
        return () => document.removeEventListener('echo:escape', handleEscape);
    });
</script>

<KeyboardHandler />

<!-- Three-column fluid canvas -->
<div class="app-container">
    <Sidebar bind:activeView />

    <main class="main-content">
        {#if activeView === "albums"}
            <AlbumGrid onSelectAlbum={(a) => transitionLayout(() => { selectedAlbum = a; })} />
        {:else if activeView === "playlists"}
            <PlaylistView onSelectPlaylist={(p) => transitionLayout(() => { selectedPlaylist = p; })} />
        {:else if activeView === "providers"}
            <div class="providers-placeholder">
                <p class="text-2">Network Providers</p>
                <p class="text-3" style="margin-top: 0.5rem; font-size: 0.875rem;">Coming soon</p>
            </div>
        {/if}
    </main>

    <RightDrawer 
        title={getDrawerTitle()} 
        isOpen={drawerOpen} 
        onClose={closeDrawer}
    >
        {#if queueOpen}
            <QueueSidebar bind:open={queueOpen} />
        {:else if selectedAlbum}
            <AlbumDetail album={selectedAlbum} onBack={closeDrawer} />
        {:else if selectedPlaylist}
            <PlaylistDetail
                playlist={selectedPlaylist}
                onBack={closeDrawer}
                onDeleted={closeDrawer}
            />
        {/if}
    </RightDrawer>
</div>

<!-- Floating player lives outside the grid, fixed bottom center -->
<PlayerBar bind:queueOpen bind:fullScreenOpen />

<FullScreenPlayer bind:isOpen={fullScreenOpen} onToggleQueue={() => transitionLayout(() => { queueOpen = !queueOpen; })} />

{#if activeView === "settings"}
    <SettingsModal onClose={() => activeView = "albums"} />
{/if}
<ToastContainer />

<style>
    .providers-placeholder {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        height: 60vh;
        text-align: center;
    }
</style>
