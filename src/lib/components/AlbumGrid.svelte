<script lang="ts">
    import { libraryStore, type Album } from "$lib/stores/library.svelte";
    import { open } from "@tauri-apps/plugin-dialog";
    import { FolderOpen, CircleNotch } from "phosphor-svelte";
    import { flip } from "svelte/animate";
    import { cubicOut } from "svelte/easing";
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
            <FolderOpen size={48} weight="thin" />
        </div>
        <h2 class="empty-heading font-headline-lg">Your library is empty</h2>
        <p class="empty-sub">Add a music folder to start your collection.</p>
        <button class="primary" onclick={handleBrowse} style="margin-top: 1.75rem;">
            <FolderOpen size={16} />
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
        <div class="header-content">
            <h1 class="lib-heading font-headline-lg text-text-main leading-tight mb-2">Your Library</h1>
            <div class="tabs">
                <button class="tab active text-[#e2a973] border-b-2 border-[#e2a973] pb-2">Albums</button>
                <button class="tab hover:text-text-main pb-2 transition-colors">Playlists</button>
                <button class="tab hover:text-text-main pb-2 transition-colors">Artists</button>
            </div>
        </div>

        <div class="lib-actions">
            {#if libraryStore.isScanning}
                <div class="scan-status">
                    <CircleNotch size={14} class="spin" />
                    <span>Scanning</span>
                </div>
            {/if}

            <button class="ghost-btn" onclick={handleBrowse}>
                <FolderOpen size={16} weight="regular" />
                Add Folder
            </button>
        </div>
    </header>

    <div class="album-grid">
        {#each libraryStore.albums as album (album.id)}
            <div style:view-transition-name="album-{album.id}">
                <AlbumCard {album} onclick={() => onSelectAlbum(album)} />
            </div>
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
        font-size: 2rem;
        font-weight: 500;
        color: var(--echo-text-1);
        letter-spacing: -0.025em;
    }

    .empty-sub {
        font-size: 1rem;
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
        font-size: 0.875rem;
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
        position: sticky;
        top: 0;
        background: rgba(5, 5, 7, 0.9);
        backdrop-filter: blur(24px);
        z-index: 20;
        padding: 3rem 2.5rem 2rem; /* pt-12 px-10 pb-8 */
        border-bottom: 1px solid rgba(255, 255, 255, 0.05); /* border-white/5 */
        margin-bottom: 0; /* Removing margin-bottom, grid has padding */
        display: flex;
        justify-content: space-between;
        align-items: flex-end;
    }

    .header-content {
        display: flex;
        flex-direction: column;
        gap: 1.5rem; /* mt-6 for tabs */
    }

    .lib-heading {
        font-size: 2.25rem; /* text-4xl */
        margin: 0;
    }

    .tabs {
        display: flex;
        gap: 1.5rem; /* gap-6 */
        font-size: 0.875rem; /* text-sm */
        letter-spacing: 0.1em; /* tracking-widest */
        text-transform: uppercase;
        font-weight: 500;
        color: var(--echo-text-2); /* text-muted */
    }

    .tab {
        background: transparent;
        border: none;
        color: inherit;
        cursor: pointer;
        padding-bottom: 0.5rem;
        padding-left: 0;
        padding-right: 0;
        border-bottom: 2px solid transparent;
        border-radius: 0;
    }

    .tab.active {
        color: var(--echo-primary);
        border-bottom-color: var(--echo-primary);
    }

    .lib-actions {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding-bottom: 0.5rem;
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
        grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
        gap: 2rem; /* gap-8 */
        padding: 2.5rem; /* p-10 */
        padding-bottom: 10rem; /* pb-40 */
    }

    /* Spinner keyframe */
    :global(.spin) {
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        from { transform: rotate(0deg); }
        to   { transform: rotate(360deg); }
    }
</style>
