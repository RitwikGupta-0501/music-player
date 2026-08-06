<script lang="ts">
    import { libraryStore, type Album } from "$lib/stores/library.svelte";
    import { open } from "@tauri-apps/plugin-dialog";
    import { FolderOpen, CircleNotch } from "phosphor-svelte";
    import { flip } from "svelte/animate";
    import { cubicOut } from "svelte/easing";
    import AlbumCard from "./AlbumCard.svelte";
    import LibraryHeader from "./LibraryHeader.svelte";

    let { activeView = $bindable("albums"), onSelectAlbum, selectedAlbumId } = $props<{ activeView?: string, onSelectAlbum: (a: Album) => void, selectedAlbumId?: number | string | null }>();

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
</script>

<!-- Library header -->
<LibraryHeader bind:activeView>
    {#snippet actions()}
        <div style="display: flex; gap: 1rem; align-items: center;">
            <span class="text-muted">{libraryStore.albums.length} Albums</span>
            
            <div style="display: flex; gap: 0.5rem; align-items: center;">
                {#if libraryStore.isScanning}
                    <div class="scan-status" style="display: flex; align-items: center; gap: 0.25rem;">
                        <CircleNotch size={14} class="spin" />
                        <span class="text-muted">Scanning</span>
                    </div>
                {/if}

                <button class="ghost" onclick={handleBrowse}>
                    <FolderOpen size={16} weight="regular" />
                    Add Folder
                </button>
            </div>
        </div>
    {/snippet}
</LibraryHeader>

{#if libraryStore.albums.length === 0 && !libraryStore.isScanning}
    <!-- Empty state / onboarding -->
    <div class="empty-state">
        <div class="empty-icon">
            <FolderOpen size={48} weight="thin" />
        </div>
        <h2 class="empty-heading font-headline-lg">Your library is empty</h2>
        <p class="empty-sub">Add a music folder to start your collection.</p>
        <button class="ghost" onclick={handleBrowse} style="margin-top: 1.75rem;">
            <FolderOpen size={16} />
            Add Music Folder
        </button>
    </div>

{:else}
    <div class="album-grid">
        {#each libraryStore.albums as album (album.id)}
            <div>
                <AlbumCard {album} selected={selectedAlbumId === album.id} onclick={() => onSelectAlbum(album)} />
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

    /* ── Album grid ── */
    .album-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
        gap: 2rem; /* gap-8 */
        padding: 2.5rem; /* p-10 */
        padding-bottom: 10rem; /* pb-40 */
    }
</style>
