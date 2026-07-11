<script lang="ts">
    import { libraryStore, type Album } from "$lib/stores/library.svelte";
    import { Play } from "phosphor-svelte";

    let { 
        album, 
        onclick 
    } = $props<{ 
        album: Album; 
        onclick: () => void 
    }>();

    let artUrl = $state<string | null>(null);

    $effect(() => {
        libraryStore.getAlbumTracks(album.id).then(tracks => {
            if (tracks && tracks.length > 0) {
                libraryStore.getArtworkUrl(tracks[0].id, tracks[0].file_path).then(url => {
                    artUrl = url;
                });
            }
        }).catch(() => {});
    });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="album-card group grid-item-smooth" {onclick}>
    <div class="art-container grid-item-smooth">
        {#if artUrl}
            <div 
                class="art-img" 
                style="background-image: url({artUrl});"
            ></div>
        {:else}
            <div class="art-placeholder">
                <!-- SVG vinyl/disc placeholder -->
                <svg viewBox="0 0 120 120" aria-hidden="true">
                    <circle cx="60" cy="60" r="58" fill="rgba(255 255 255 / 0.03)" stroke="rgba(255 255 255 / 0.06)" stroke-width="1" />
                    <circle cx="60" cy="60" r="40" fill="rgba(255 255 255 / 0.02)" stroke="rgba(255 255 255 / 0.05)" stroke-width="0.75" />
                    <circle cx="60" cy="60" r="22" fill="rgba(255 255 255 / 0.02)" stroke="rgba(255 255 255 / 0.04)" stroke-width="0.75" />
                    <circle cx="60" cy="60" r="7" fill="rgba(255 255 255 / 0.08)" />
                    <circle cx="60" cy="60" r="3" fill="rgba(255 255 255 / 0.12)" />
                </svg>
            </div>
        {/if}

        <div class="play-overlay">
            <div class="play-btn">
                <Play weight="fill" size={32} />
            </div>
        </div>
    </div>
    
    <h3 class="card-title">{album.title}</h3>
    <p class="card-artist">{album.artist || "Unknown Artist"}</p>
</div>

<style>
    .grid-item-smooth {
        transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .album-card {
        cursor: pointer;
    }

    .art-container {
        width: 100%;
        aspect-ratio: 1;
        border-radius: 1.5rem; /* rounded-[1.5rem] */
        background-color: #27272a; /* bg-zinc-800 */
        border: 1px solid rgba(255, 255, 255, 0.1);
        overflow: hidden;
        box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.5), 0 4px 6px -4px rgba(0, 0, 0, 0.5); /* shadow-lg */
        margin-bottom: 1rem; /* mb-4 */
        position: relative;
    }

    .art-img {
        width: 100%;
        height: 100%;
        background-size: cover;
        background-position: center;
        transition: transform 0.7s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .album-card:hover .art-img {
        transform: scale(1.05); /* group-hover:scale-105 */
    }

    .art-placeholder {
        width: 100%;
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .art-placeholder svg {
        width: 60%;
        height: 60%;
        opacity: 0.6;
    }

    .play-overlay {
        position: absolute;
        inset: 0;
        background-color: rgba(0, 0, 0, 0.4);
        backdrop-filter: blur(4px);
        opacity: 0;
        transition: opacity 0.3s ease;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .album-card:hover .play-overlay {
        opacity: 1;
    }

    .play-btn {
        width: 56px; /* w-14 */
        height: 56px; /* h-14 */
        border-radius: 9999px; /* rounded-full */
        background-color: var(--echo-primary-dark); /* bg-[#B58E62] */
        display: flex;
        align-items: center;
        justify-content: center;
        color: #000;
        box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.5), 0 4px 6px -4px rgba(0, 0, 0, 0.5);
    }

    .card-title {
        font-family: var(--echo-font-body); /* Should not be headline-lg in grid */
        font-size: 1rem; /* text-base */
        font-weight: 500;
        color: var(--echo-text-1);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        margin: 0;
    }

    .card-artist {
        font-family: var(--echo-font-body);
        font-size: 0.875rem; /* text-sm */
        color: var(--echo-text-2);
        margin-top: 0.25rem; /* mt-1 */
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
</style>
