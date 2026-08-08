<script lang="ts">
    import { libraryStore, type Playlist } from "$lib/stores/library.svelte";
    import { flip } from "svelte/animate";
    import { cubicOut } from "svelte/easing";
    import { Plus, ListPlus } from "phosphor-svelte";
    import { createVirtualizer } from "@tanstack/svelte-virtual";
    import { onMount } from "svelte";
    import LibraryHeader from "./LibraryHeader.svelte";
    import PromptModal from "./PromptModal.svelte";

    let { activeView = $bindable("playlists"), onSelectPlaylist } = $props<{ activeView?: string, onSelectPlaylist: (p: Playlist) => void }>();

    let mosaics = $state<Record<number, string[]>>({});
    let promptOpen = $state(false);

    let containerWidth = $state(0);
    let cols = $derived(Math.max(1, Math.floor((containerWidth + 32) / 222))); // 190px + 32px gap

    let rows = $derived.by(() => {
        const result = [];
        const playlists = libraryStore.playlists;
        for (let i = 0; i < playlists.length; i += cols) {
            result.push(playlists.slice(i, i + cols));
        }
        return result;
    });

    let mainContent = $state<HTMLElement | null>(null);
    onMount(() => {
        mainContent = document.querySelector('.main-content');
    });

    let virtStore = $derived.by(() => {
        const mc = mainContent;
        const rowCount = rows.length;
        const cw = containerWidth;
        const cCount = cols;
        const colWidth = cCount > 0 ? (cw - (cCount - 1) * 32) / cCount : 0;
        const rowHeight = colWidth > 0 ? colWidth + 52 + 32 : 320; // card + text + gap

        return createVirtualizer({
            count: rowCount,
            getScrollElement: () => mc,
            estimateSize: () => rowHeight,
            overscan: 5,
        });
    });

    async function handleCreate(name: string) {
        if (!name || !name.trim()) return;
        await libraryStore.createPlaylist(name.trim());
        promptOpen = false;
        await loadMosaics();
    }

    function handleCreatePrompt() {
        promptOpen = true;
    }

    async function loadMosaics() {
        const entries = await Promise.all(
            libraryStore.playlists.map(async p => [p.id, await libraryStore.getPlaylistArtworkMosaic(p.id)] as const)
        );
        mosaics = Object.fromEntries(entries);
    }

    // Reactively load mosaics when playlists change
    $effect(() => {
        if (libraryStore.playlists) {
            loadMosaics();
        }
    });
</script>

<LibraryHeader bind:activeView>
    {#snippet actions()}
        <div style="display: flex; gap: 1rem; align-items: center;">
            <span class="text-muted">{libraryStore.playlists.length} Playlists</span>
            <div style="display: flex; gap: 0.5rem;">
                <button class="ghost" onclick={handleCreatePrompt}>
                    <Plus size={16} />
                    New Playlist
                </button>
            </div>
        </div>
    {/snippet}
</LibraryHeader>

{#if libraryStore.playlists.length === 0}
    <div class="empty-state">
        <div class="empty-icon">
            <ListPlus size={48} weight="thin" />
        </div>
        <h2 class="empty-heading font-headline-lg">You have no playlists</h2>
        <p class="empty-sub">Create a playlist to organize your favorite tracks.</p>
        <button class="ghost" onclick={handleCreatePrompt} style="margin-top: 1.75rem;">
            <Plus size={16} />
            Create Playlist
        </button>
    </div>
{:else}
    <div class="playlist-grid">
        <div bind:clientWidth={containerWidth} style="position: relative; width: 100%; height: {$virtStore.getTotalSize()}px;">
            {#each $virtStore.getVirtualItems() as virtualRow (virtualRow.index)}
                {@const r = virtualRow.index}
                {@const rowPlaylists = rows[r]}
                <div class="virtual-row" style="position: absolute; top: 0; left: 0; width: 100%; transform: translateY({virtualRow.start}px); grid-template-columns: repeat({cols}, minmax(0, 1fr));">
                    {#each rowPlaylists as playlist (playlist.id)}
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <div 
                            class="playlist-card glass-panel" 
                            onclick={() => onSelectPlaylist(playlist)}
                        >
                            <div class="art">
                                {#if mosaics[playlist.id] && mosaics[playlist.id].length >= 4}
                                    <div class="mosaic">
                                        <img src={mosaics[playlist.id][0]} alt="Cover" />
                                        <img src={mosaics[playlist.id][1]} alt="Cover" />
                                        <img src={mosaics[playlist.id][2]} alt="Cover" />
                                        <img src={mosaics[playlist.id][3]} alt="Cover" />
                                    </div>
                                {:else if mosaics[playlist.id] && mosaics[playlist.id].length > 0}
                                    <img src={mosaics[playlist.id][0]} alt="Cover" style="width: 100%; height: 100%; object-fit: cover;" />
                                {:else}
                                    <div class="icon">📝</div>
                                {/if}
                            </div>
                            <h3 style="margin-top: 0.5rem; text-align: center; width: 100%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">{playlist.name}</h3>
                        </div>
                    {/each}
                </div>
            {/each}
        </div>
    </div>
{/if}

{#if promptOpen}
    <PromptModal
        title="Create Playlist"
        defaultValue="New Playlist"
        onSubmit={handleCreate}
        onClose={() => promptOpen = false}
    />
{/if}

<style>
    .playlist-grid {
        padding: 2.5rem;
        padding-bottom: 10rem;
    }
    .virtual-row {
        display: grid;
        gap: 2rem;
    }
    .playlist-card {
        display: flex;
        flex-direction: column;
        cursor: pointer;
        transition: transform 0.22s cubic-bezier(0.34, 1.56, 0.64, 1);
    }
    .playlist-card:hover {
        transform: translateY(-4px);
    }
    .art {
        width: 100%;
        aspect-ratio: 1;
        background-color: #27272a;
        border-radius: 1.5rem;
        overflow: hidden;
        display: flex;
        justify-content: center;
        align-items: center;
        border: 1px solid rgba(255, 255, 255, 0.1);
        box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.5);
    }
    .mosaic {
        display: grid;
        grid-template-columns: 1fr 1fr;
        grid-template-rows: 1fr 1fr;
        width: 100%;
        height: 100%;
    }
    .mosaic img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }
    .icon {
        font-size: 3rem;
        opacity: 0.5;
    }
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
</style>
