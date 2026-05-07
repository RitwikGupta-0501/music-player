<script lang="ts">
    import { libraryStore, type Album, type LocalTrack } from "$lib/stores/library.svelte";
    import { audioStore } from "$lib/stores/audio.svelte";
    import { onMount } from "svelte";

    let { album, onBack } = $props<{ album: Album, onBack: () => void }>();
    
    let tracks = $state<LocalTrack[]>([]);
    let artUrl = $state<string | null>(null);

    onMount(async () => {
        tracks = await libraryStore.getAlbumTracks(album.id);
        if (tracks.length > 0) {
            artUrl = await libraryStore.getArtworkUrl(tracks[0].id, tracks[0].file_path);
        }
    });

    function playTrack(path: string) {
        audioStore.load(path);
    }
</script>

<button class="ghost" style="margin-bottom: 2rem; padding: 0.5rem 0;" onclick={onBack}>
    ← Back to Library
</button>

<div class="album-header">
    <div class="art glass-panel">
        {#if artUrl}
            <img src={artUrl} alt={album.title} />
        {:else}
            <div class="placeholder">🎵</div>
        {/if}
    </div>
    <div class="info">
        <h1 style="font-size: 3rem; margin-bottom: 0.5rem;">{album.title}</h1>
        <h2 class="text-muted" style="font-size: 1.5rem;">{album.artist || "Unknown Artist"}</h2>
    </div>
</div>

<div class="track-list" style="margin-top: 3rem;">
    {#each tracks as track, i}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="track-card" onclick={() => playTrack(track.file_path)}>
            <div style="display: flex; gap: 1rem; align-items: center;">
                <span class="text-muted" style="width: 20px; text-align: right;">{track.track_number || i + 1}</span>
                <strong>{track.title}</strong>
            </div>
            <span class="text-muted">▶</span>
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
        display: flex;
        justify-content: center;
        align-items: center;
        background: rgba(0,0,0,0.2);
        overflow: hidden;
    }
    .art img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }
    .placeholder {
        font-size: 5rem;
        opacity: 0.5;
    }
</style>
