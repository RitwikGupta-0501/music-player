<script lang="ts">
    import { libraryStore, type Album } from "$lib/stores/library.svelte";

    let { album, onclick } = $props<{ album: Album; onclick: () => void }>();

    let artUrl = $state<string | null>(null);

    $effect(() => {
        libraryStore.getAlbumTracks(album.id).then(tracks => {
            if (tracks && tracks.length > 0) {
                libraryStore.getArtworkUrl(tracks[0].id, tracks[0].file_path).then(url => {
                    artUrl = url;
                });
            }
        }).catch(() => {
            // leave artUrl as null, placeholder renders
        });
    });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="album-card" {onclick}>
    <div class="art-wrap">
        {#if artUrl}
            <img src={artUrl} alt={album.title} loading="lazy" />
        {:else}
            <!-- SVG vinyl/disc placeholder - no emoji -->
            <svg class="art-placeholder" viewBox="0 0 120 120" aria-hidden="true">
                <circle cx="60" cy="60" r="58" fill="rgba(255 255 255 / 0.03)" stroke="rgba(255 255 255 / 0.06)" stroke-width="1" />
                <circle cx="60" cy="60" r="40" fill="rgba(255 255 255 / 0.02)" stroke="rgba(255 255 255 / 0.05)" stroke-width="0.75" />
                <circle cx="60" cy="60" r="22" fill="rgba(255 255 255 / 0.02)" stroke="rgba(255 255 255 / 0.04)" stroke-width="0.75" />
                <circle cx="60" cy="60" r="7" fill="rgba(255 255 255 / 0.08)" />
                <circle cx="60" cy="60" r="3" fill="rgba(255 255 255 / 0.12)" />
            </svg>
        {/if}
    </div>
    <div class="card-info">
        <p class="card-title">{album.title}</p>
        <p class="card-artist">{album.artist || "Unknown Artist"}</p>
    </div>
</div>

<style>
    .album-card {
        display: flex;
        flex-direction: column;
        border-radius: 10px;
        overflow: hidden;
        cursor: pointer;
        background: var(--echo-surface);
        border: 1px solid var(--echo-border);
        transition:
            transform 0.22s cubic-bezier(0.34, 1.56, 0.64, 1),
            border-color 0.15s ease,
            box-shadow 0.22s ease;
    }

    .album-card:hover {
        transform: translateY(-5px);
        border-color: var(--echo-border-medium);
        box-shadow: 0 14px 40px rgba(0 0 0 / 0.55);
    }

    .album-card:active {
        transform: translateY(-2px) scale(0.985);
        transition-duration: 0.08s;
    }

    /* ── Artwork ── */
    .art-wrap {
        aspect-ratio: 1;
        overflow: hidden;
        background: var(--echo-raised);
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .art-wrap img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
        transition: transform 0.35s ease;
    }

    .album-card:hover .art-wrap img {
        transform: scale(1.05);
    }

    .art-placeholder {
        width: 72%;
        height: 72%;
        opacity: 0.7;
    }

    /* ── Info strip ── */
    .card-info {
        padding: 0.7rem 0.75rem;
        border-top: 1px solid var(--echo-border);
    }

    .card-title {
        font-size: 0.775rem;
        font-weight: 500;
        color: var(--echo-text-1);
        letter-spacing: -0.01em;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        margin-bottom: 2px;
    }

    .card-artist {
        font-size: 0.7rem;
        color: var(--echo-text-2);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
</style>
