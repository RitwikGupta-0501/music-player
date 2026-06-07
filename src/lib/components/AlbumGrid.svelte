<script lang="ts">
    import { libraryStore, type Album } from "$lib/stores/library.svelte";
    import { open } from "@tauri-apps/plugin-dialog";
    import { FolderOpen, Loader2 } from "lucide-svelte";
    import AlbumCard from "./AlbumCard.svelte";

    let { onSelectAlbum } = $props<{ onSelectAlbum: (a: Album) => void }>();
    
    let scanDirectory = $state("");

    async function handleBrowse() {
        const selected = await open({
            directory: true,
            multiple: false,
            title: "Select Music Folder",
        });
        if (selected) {
            await libraryStore.scanDirectory(selected as string);
        }
    }

    async function handleManualScan() {
        if (scanDirectory.trim()) {
            await libraryStore.scanDirectory(scanDirectory.trim());
        }
    }
</script>

{#if libraryStore.albums.length === 0 && !libraryStore.isScanning}
    <!-- Empty State / Onboarding -->
    <div class="empty-state">
        <div class="empty-icon">
            <FolderOpen size={64} strokeWidth={1} />
        </div>
        <h2 style="margin-bottom: 0.5rem;">Your library is empty</h2>
        <p class="text-muted" style="margin-bottom: 2rem; max-width: 400px; text-align: center;">
            Add a folder to start your collection.
        </p>
        <button class="primary" onclick={handleBrowse}>
            <FolderOpen size={16} />
            Add Music Folder
        </button>
        <div class="manual-scan">
            <span class="text-muted" style="font-size: 0.8rem;">or enter a path manually:</span>
            <div style="display: flex; gap: 0.5rem; margin-top: 0.5rem;">
                <input
                    type="text"
                    bind:value={scanDirectory}
                    placeholder="/home/user/Music"
                    disabled={libraryStore.isScanning}
                    spellcheck="false"
                    style="width: 300px;"
                />
                <button class="primary" onclick={handleManualScan} disabled={libraryStore.isScanning || !scanDirectory.trim()}>
                    Scan
                </button>
            </div>
        </div>
    </div>
{:else}
    <!-- Library Header -->
    <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 2rem;">
        <div>
            <h2 class="text-cyan" style="font-size: 2.5rem; margin-bottom: 0.5rem;">Library</h2>
            <span class="text-muted">{libraryStore.albums.length} Albums</span>
        </div>
        
        <div style="display: flex; gap: 0.5rem; align-items: center;">
            {#if libraryStore.isScanning}
                <div class="scanning-indicator">
                    <Loader2 size={16} class="spin" />
                    <span class="text-muted">Scanning...</span>
                </div>
            {/if}
            <button class="ghost" onclick={handleBrowse} title="Add Music Folder" style="display: flex; align-items: center; gap: 0.5rem;">
                <FolderOpen size={16} />
                Add Folder
            </button>
        </div>
    </div>

    <div class="album-grid">
        {#each libraryStore.albums as album}
            <AlbumCard {album} onclick={() => onSelectAlbum(album)} />
        {/each}
    </div>
{/if}

<style>
    .album-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        gap: 1.5rem;
    }

    .empty-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        height: 70vh;
        gap: 0.5rem;
    }

    .empty-icon {
        color: var(--color-chalk-muted);
        opacity: 0.4;
        margin-bottom: 1rem;
    }

    .empty-state button.primary {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.85rem 1.5rem;
        font-size: 1rem;
    }

    .manual-scan {
        margin-top: 2rem;
        display: flex;
        flex-direction: column;
        align-items: center;
    }

    .scanning-indicator {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    :global(.spin) {
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        from { transform: rotate(0deg); }
        to { transform: rotate(360deg); }
    }
</style>
