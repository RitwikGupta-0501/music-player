<script lang="ts">
    import { libraryStore, type Playlist, type LocalTrack } from "$lib/stores/library.svelte";
    import { audioStore } from "$lib/stores/audio.svelte";
    import { toastStore } from "$lib/stores/toast.svelte";

    let { playlist, onBack, onDeleted } = $props<{ playlist: Playlist, onBack: () => void, onDeleted: () => void }>();
    
    let tracks = $state<LocalTrack[]>([]);
    let artUrls = $state<string[]>([]);
    let isEditingName = $state(false);
    let editName = $state("");
    let draggedIndex = $state<number | null>(null);

    async function loadData() {
        tracks = await libraryStore.getPlaylistTracks(playlist.id);
        artUrls = await libraryStore.getPlaylistArtworkMosaic(playlist.id);
    }

    $effect(() => {
        loadData();
        if (!isEditingName) {
            editName = playlist.name;
        }
    });

    async function playAll() {
        if (tracks.length === 0) return;
        await audioStore.setQueue(
            tracks.map(t => ({
                id: t.id,
                title: t.title,
                artist: t.artist,
                file_path: t.file_path,
            })),
            0
        );
        await audioStore.play();
    }

    async function playTrack(index: number) {
        if (audioStore.queue.length > 0) {
            const t = tracks[index];
            const trackPayload = {
                id: t.id,
                title: t.title,
                artist: t.artist,
                file_path: t.file_path,
            };

            if (audioStore.trackClickBehavior === "interrupt") {
                await audioStore.playInterrupt(trackPayload);
                return;
            } else if (audioStore.trackClickBehavior === "append") {
                await audioStore.addToQueue(trackPayload);
                toastStore.info("Added to queue");
                return;
            }
        }

        await audioStore.setQueue(
            tracks.map(t => ({
                id: t.id,
                title: t.title,
                artist: t.artist,
                file_path: t.file_path,
            })),
            index
        );
        await audioStore.play();
    }

    async function removeTrack(e: Event, trackId: number) {
        e.stopPropagation();
        await libraryStore.removeFromPlaylist(playlist.id, trackId);
        await loadData();
    }

    async function deletePlaylist() {
        if (confirm("Are you sure you want to delete this playlist?")) {
            await libraryStore.deletePlaylist(playlist.id);
            onDeleted();
        }
    }

    async function saveName() {
        if (editName.trim() && editName !== playlist.name) {
            await libraryStore.renamePlaylist(playlist.id, editName);
        }
        isEditingName = false;
    }

    // Drag and Drop
    function handleDragStart(e: DragEvent, index: number) {
        draggedIndex = index;
        if (e.dataTransfer) {
            e.dataTransfer.effectAllowed = 'move';
            e.dataTransfer.setData('text/plain', index.toString());
        }
    }

    function handleDragOver(e: DragEvent) {
        e.preventDefault();
        if (e.dataTransfer) {
            e.dataTransfer.dropEffect = 'move';
        }
    }

    async function handleDrop(e: DragEvent, dropIndex: number) {
        e.preventDefault();
        if (draggedIndex === null || draggedIndex === dropIndex) return;

        // Visual update immediately
        const draggedTrack = tracks[draggedIndex];
        tracks.splice(draggedIndex, 1);
        tracks.splice(dropIndex, 0, draggedTrack);
        tracks = [...tracks]; // trigger reactivity

        // Backend update (SQLite positions are 1-based based on our DB logic)
        const fromPos = draggedIndex + 1;
        const toPos = dropIndex + 1;
        
        await libraryStore.reorderPlaylistTrack(playlist.id, fromPos, toPos);
        draggedIndex = null;
    }
</script>

<button class="ghost" style="margin-bottom: 2rem; padding: 0.5rem 0;" onclick={onBack}>
    ← Back to Playlists
</button>

<div class="album-header">
    <div class="art glass-panel" style="padding: 0;">
        {#if artUrls.length >= 4}
            <div class="mosaic">
                <img src={artUrls[0]} alt="Cover" />
                <img src={artUrls[1]} alt="Cover" />
                <img src={artUrls[2]} alt="Cover" />
                <img src={artUrls[3]} alt="Cover" />
            </div>
        {:else if artUrls.length > 0}
            <img src={artUrls[0]} alt="Cover" style="width: 100%; height: 100%; object-fit: cover;" />
        {:else}
            <div class="placeholder" style="display: flex; justify-content: center; align-items: center; width: 100%; height: 100%;">📝</div>
        {/if}
    </div>
    <div class="info" style="flex: 1;">
        {#if isEditingName}
            <div style="display: flex; gap: 1rem; margin-bottom: 0.5rem;">
                <input type="text" bind:value={editName} style="font-size: 2.5rem; background: transparent; border: 1px solid var(--glass-border); color: white; padding: 0.5rem; flex: 1;" />
                <button class="primary" onclick={saveName}>Save</button>
            </div>
        {:else}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <h1 style="font-size: 3rem; margin-bottom: 0.5rem; cursor: text;" onclick={() => isEditingName = true} title="Click to rename">
                {playlist.name} <span style="font-size: 1.5rem; opacity: 0.5;">✎</span>
            </h1>
        {/if}
        <div style="display: flex; gap: 1rem; align-items: center; margin-top: 1rem;">
            <button class="primary" onclick={playAll} disabled={tracks.length === 0}>Play Playlist</button>
            <button class="ghost text-danger" onclick={deletePlaylist} style="color: var(--color-danger); border-color: var(--color-danger);">Delete Playlist</button>
        </div>
    </div>
</div>

<div class="track-list" style="margin-top: 3rem;">
    {#if tracks.length === 0}
        <p class="text-muted" style="text-align: center; margin-top: 2rem;">This playlist is empty.</p>
    {/if}
    {#each tracks as track, i}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_interactive_supports_focus -->
        <div class="track-card" 
             class:now-playing={audioStore.currentTrack === track.file_path}
             draggable="true" 
             ondragstart={(e) => handleDragStart(e, i)}
             ondragover={handleDragOver}
             ondrop={(e) => handleDrop(e, i)}
             onclick={() => playTrack(i)}
             style={draggedIndex === i ? "opacity: 0.5;" : ""}
             role="button">
            <div style="display: flex; gap: 1rem; align-items: center; flex: 1;">
                <span class="text-muted" style="cursor: grab;" title="Drag to reorder">⠿</span>
                <span class="text-muted" style="width: 20px; text-align: right;">{i + 1}</span>
                <div style="display: flex; flex-direction: column;">
                    <strong>{track.title}</strong>
                    <span class="text-muted" style="font-size: 0.85rem;">{track.artist || "Unknown Artist"}</span>
                </div>
            </div>
            <button class="ghost text-danger" style="padding: 0.2rem 0.5rem; border: none; font-size: 1.2rem;" onclick={(e) => removeTrack(e, track.id)} title="Remove from playlist">✕</button>
        </div>
    {/each}
</div>

<style>
    .album-header {
        display: flex;
        gap: 2rem;
        align-items: flex-end;
    }
    .art {
        width: 250px;
        height: 250px;
        flex-shrink: 0;
        background: rgba(0,0,0,0.2);
        overflow: hidden;
        border-radius: 12px;
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
    .placeholder {
        font-size: 5rem;
        opacity: 0.5;
    }
    .text-danger {
        color: #ff6b6b;
    }
</style>
