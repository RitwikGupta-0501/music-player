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
    <!-- Empty state / onboarding -->
    <div class="empty-state">
        <div class="empty-icon">
            <FolderOpen size={48} strokeWidth={1} />
        </div>
        <h2 class="empty-heading">Your library is empty</h2>
        <p class="empty-sub">Add a music folder to start your collection.</p>
        <button class="primary" onclick={handleBrowse} style="margin-top: 1.75rem;">
            <FolderOpen size={15} />
            Add Music Folder
        </button>
        <div class="manual-scan">
            <span class="or-label">or enter a path directly</span>
            <div class="path-row">
                <input
                    type="text"
                    bind:value={scanDirectory}
                    placeholder="/home/user/Music"
                    disabled={libraryStore.isScanning}
                    spellcheck="false"
                />
                <button
                    class="primary"
                    onclick={handleManualScan}
                    disabled={libraryStore.isScanning || !scanDirectory.trim()}
                >
                    Scan
                </button>
            </div>
        </div>
    </div>

{:else}
    <!-- Library header -->
    <header class="lib-header">
        <div class="lib-heading-group">
            <h1 class="lib-heading">Library</h1>
            <span class="lib-count">{libraryStore.albums.length} albums</span>
        </div>

        <div class="lib-actions">
            {#if libraryStore.isScanning}
                <div class="scan-status">
                    <Loader2 size={13} class="spin" />
                    <span>Scanning</span>
                </div>
            {/if}
            <button class="ghost" onclick={handleBrowse}>
                <FolderOpen size={14} />
                Add Folder
            </button>
        </div>
    </header>

    <div class="album-grid">
        {#each libraryStore.albums as album}
            <AlbumCard {album} onclick={() => onSelectAlbum(album)} />
        {/each}
    </div>
{/if}

<style>
    /* ── Empty state ── */
    .empty-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        min-height: 65vh;
        text-align: center;
        gap: 0.5rem;
    }

    .empty-icon {
        color: var(--echo-text-3);
        margin-bottom: 1.25rem;
        opacity: 0.6;
    }

    .empty-heading {
        font-size: 1.5rem;
        font-weight: 600;
        color: var(--echo-text-1);
        letter-spacing: -0.025em;
    }

    .empty-sub {
        font-size: 0.875rem;
        color: var(--echo-text-2);
        max-width: 280px;
        line-height: 1.6;
    }

    .manual-scan {
        margin-top: 2.25rem;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.6rem;
    }

    .or-label {
        font-size: 0.75rem;
        color: var(--echo-text-3);
    }

    .path-row {
        display: flex;
        gap: 0.5rem;
        align-items: center;
    }

    .path-row input {
        width: 280px;
    }

    /* ── Library header ── */
    .lib-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-end;
        margin-bottom: 2rem;
    }

    .lib-heading-group {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .lib-heading {
        font-size: 2rem;
        font-weight: 700;
        letter-spacing: -0.04em;
        color: var(--echo-text-1);
        line-height: 1;
    }

    .lib-count {
        font-size: 0.78rem;
        color: var(--echo-text-3);
        font-variant-numeric: tabular-nums;
    }

    .lib-actions {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .scan-status {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        font-size: 0.78rem;
        color: var(--echo-text-3);
    }

    /* ── Album grid ── */
    .album-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(178px, 1fr));
        gap: 1.125rem;
    }

    /* Spinner keyframe */
    :global(.spin) {
        animation: spin 1.2s linear infinite;
    }

    @keyframes spin {
        from { transform: rotate(0deg); }
        to   { transform: rotate(360deg); }
    }
</style>
