<script lang="ts">
    import { audioStore } from "$lib/stores/audio.svelte";
    import { libraryStore } from "$lib/stores/library.svelte";
    import { CornersIn, Play, Pause, SkipBack, SkipForward, Playlist } from "phosphor-svelte";
    import { slide, fade } from "svelte/transition";

    let { 
        isOpen = $bindable(false),
        onToggleQueue
    } = $props<{ 
        isOpen: boolean;
        onToggleQueue: () => void;
    }>();

    let artUrl = $state<string | null>(null);

    $effect(() => {
        const track = audioStore.currentQueueTrack;
        if (track && track.source.type === 'Local' && track.source.track_id && track.source.file_path) {
            libraryStore.getArtworkUrl(track.source.track_id, track.source.file_path).then(url => {
                artUrl = url;
            }).catch(() => {
                artUrl = null;
            });
        } else if (track && track.source.type === 'Remote' && track.source.cover_art_url) {
            artUrl = track.source.cover_art_url;
        } else {
            artUrl = null;
        }
    });

    function togglePlay() {
        if (audioStore.playbackState === "Playing") {
            audioStore.pause();
        } else {
            audioStore.play();
        }
    }
</script>

{#if isOpen}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div 
        class="fullscreen-overlay" 
        transition:fade={{ duration: 300 }}
    >
        <!-- Background Blur -->
        {#if artUrl}
            <div class="bg-blur" style="background-image: url({artUrl});"></div>
        {/if}
        <div class="bg-dim"></div>

        <div class="fs-header">
            <button class="icon-btn" onclick={() => isOpen = false} title="Exit Fullscreen">
                <CornersIn size={24} weight="bold" />
            </button>
        </div>

        <div class="fs-content">
            <div class="fs-art-container">
                {#if artUrl}
                    <img src={artUrl} alt="Album Art" class="fs-art shadow-xl" />
                {:else}
                    <div class="fs-art fs-placeholder shadow-xl">
                        <div class="record-ring"></div>
                    </div>
                {/if}
            </div>

            <div class="fs-controls">
                <div class="fs-info">
                    <h1 class="fs-title">{audioStore.currentQueueTrack?.title || audioStore.currentTrack || "Not Playing"}</h1>
                    <p class="fs-artist">{audioStore.currentQueueTrack?.artist || "Unknown Artist"}</p>
                </div>

                <div class="fs-buttons">
                    <button class="icon-btn" onclick={() => audioStore.previous()}>
                        <SkipBack size={32} weight="fill" />
                    </button>
                    <button class="play-btn" onclick={togglePlay}>
                        {#if audioStore.playbackState === "Playing"}
                            <Pause size={40} weight="fill" color="#000" />
                        {:else}
                            <Play size={40} weight="fill" color="#000" />
                        {/if}
                    </button>
                    <button class="icon-btn" onclick={() => audioStore.next()}>
                        <SkipForward size={32} weight="fill" />
                    </button>
                </div>
            </div>
        </div>
    </div>
{/if}

<style>
    .fullscreen-overlay {
        position: fixed;
        inset: 0;
        z-index: 1000;
        display: flex;
        flex-direction: column;
        background-color: var(--echo-void);
        color: var(--echo-text-1);
    }

    .bg-blur {
        position: absolute;
        inset: -10%;
        background-size: cover;
        background-position: center;
        filter: blur(80px) brightness(0.4) saturate(1.5);
        opacity: 0.7;
        z-index: -2;
    }

    .bg-dim {
        position: absolute;
        inset: 0;
        background: linear-gradient(to bottom, rgba(5,5,7,0.3) 0%, rgba(5,5,7,0.9) 100%);
        z-index: -1;
    }

    .fs-header {
        padding: 2rem;
        display: flex;
        justify-content: flex-end;
    }

    .fs-content {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 2rem;
        gap: 4rem;
    }

    .fs-art-container {
        width: min(50vh, 500px);
        aspect-ratio: 1;
    }

    .fs-art {
        width: 100%;
        height: 100%;
        object-fit: cover;
        border-radius: 1.5rem;
        box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.7);
    }

    .fs-placeholder {
        background: var(--echo-surface);
        display: flex;
        align-items: center;
        justify-content: center;
        border: 1px solid var(--echo-border);
    }

    .record-ring {
        width: 60%;
        height: 60%;
        border-radius: 50%;
        border: 4px solid var(--echo-border-medium);
        position: relative;
    }

    .fs-controls {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 2rem;
        text-align: center;
    }

    .fs-title {
        font-family: var(--echo-font-heading);
        font-size: 3rem;
        font-weight: 600;
        margin-bottom: 0.5rem;
        letter-spacing: -0.02em;
    }

    .fs-artist {
        font-family: var(--echo-font-body);
        font-size: 1.25rem;
        color: var(--echo-primary);
        text-transform: uppercase;
        letter-spacing: 0.1em;
    }

    .fs-buttons {
        display: flex;
        align-items: center;
        gap: 3rem;
    }

    .icon-btn {
        background: transparent;
        border: none;
        color: var(--echo-text-2);
        cursor: pointer;
        transition: color 0.2s, transform 0.2s;
    }

    .icon-btn:hover {
        color: var(--echo-text-1);
        transform: scale(1.1);
    }

    .play-btn {
        width: 80px;
        height: 80px;
        border-radius: 50%;
        background: var(--echo-primary);
        border: none;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: transform 0.2s, box-shadow 0.2s;
        box-shadow: 0 10px 25px rgba(226, 169, 115, 0.4);
    }

    .play-btn:hover {
        transform: scale(1.05);
        box-shadow: 0 15px 35px rgba(226, 169, 115, 0.6);
    }
</style>
