<script lang="ts">
    import { libraryStore, type Album } from "$lib/stores/library.svelte";
    import { onMount } from "svelte";

    let { album, onclick } = $props<{ album: Album, onclick: () => void }>();
    
    let artUrl = $state<string | null>(null);

    onMount(async () => {
        // If the album already has an extracted path, we could use it, 
        // but for safety, we'll try to find a track in this album and get its artwork.
        // Wait, the new schema has cover_art_path in the Album? No, extract_and_cache_artwork takes a track_id and file_path.
        // So we need to fetch one track for this album to get the artwork.
        try {
            const tracks = await libraryStore.getAlbumTracks(album.id);
            if (tracks && tracks.length > 0) {
                artUrl = await libraryStore.getArtworkUrl(tracks[0].id, tracks[0].file_path);
            }
        } catch (e) {
            console.error("Failed to load artwork for album", album.id);
        }
    });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="album-card glass-panel" {onclick}>
    <div class="art-container">
        {#if artUrl}
            <img src={artUrl} alt={album.title} />
        {:else}
            <div class="placeholder">🎵</div>
        {/if}
    </div>
    <div class="info">
        <h4>{album.title}</h4>
        <span class="text-muted">{album.artist || "Unknown Artist"}</span>
    </div>
</div>

<style>
    .album-card {
        display: flex;
        flex-direction: column;
        overflow: hidden;
        cursor: pointer;
        transition: transform 0.2s, box-shadow 0.2s;
        padding: 0;
    }
    .album-card:hover {
        transform: scale(1.02);
        box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
    }
    .art-container {
        width: 100%;
        aspect-ratio: 1;
        background: rgba(0, 0, 0, 0.2);
        display: flex;
        justify-content: center;
        align-items: center;
        border-bottom: 1px solid var(--glass-border);
    }
    .art-container img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }
    .placeholder {
        font-size: 3rem;
        opacity: 0.5;
    }
    .info {
        padding: 1rem;
    }
    .info h4 {
        margin: 0 0 0.25rem 0;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
</style>
