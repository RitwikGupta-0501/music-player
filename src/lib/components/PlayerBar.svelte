<script lang="ts">
    import { audioStore } from "$lib/stores/audio.svelte";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { libraryStore } from "$lib/stores/library.svelte";
    import {
        Play,
        Pause,
        SkipBack,
        SkipForward,
        Shuffle,
        Repeat,
        RepeatOnce,
        SpeakerHigh,
        SpeakerX,
        ListNumbers,
        CornersOut,
    } from "phosphor-svelte";
    import { transitionLayout } from "$lib/utils/transitions";

    let { queueOpen = $bindable(false), fullScreenOpen = $bindable(false) } =
        $props<{
            queueOpen?: boolean;
            fullScreenOpen?: boolean;
        }>();

    /* ── Album art for now-playing track ── */
    let playerArtUrl = $state<string | null>(null);

    $effect(() => {
        const track = audioStore.currentQueueTrack;
        if (
            track &&
            track.source.type === "Local" &&
            track.source.track_id &&
            track.source.file_path
        ) {
            libraryStore
                .getArtworkUrl(track.source.track_id, track.source.file_path)
                .then((url) => {
                    playerArtUrl = url;
                })
                .catch(() => {
                    playerArtUrl = null;
                });
        } else if (
            track &&
            track.source.type === "Remote" &&
            track.source.cover_art_url
        ) {
            playerArtUrl = track.source.cover_art_url;
        } else {
            playerArtUrl = null;
        }
    });

    /* ── Playback helpers ── */
    function handlePlayPause() {
        if (audioStore.playbackState === "Playing") {
            audioStore.pause();
        } else {
            audioStore.play();
        }
    }

    // -- Seek State --
    let isSeeking = $state(false);
    let seekProgress = $state(0);

    let progress = $derived(
        audioStore.duration > 0
            ? (audioStore.currentTime / audioStore.duration) * 100
            : 0,
    );

    let displayProgress = $derived(isSeeking ? seekProgress : progress);

    let volumeProgress = $derived(
        audioStore.isMuted ? 0 : audioStore.volume * 100,
    );

    function formatTime(seconds: number): string {
        if (isNaN(seconds) || seconds < 0) return "0:00";
        const m = Math.floor(seconds / 60);
        const s = Math.floor(seconds % 60);
        return `${m}:${s.toString().padStart(2, "0")}`;
    }

    function handleSeekPointerDown() {
        if (audioStore.duration <= 0) return;
        isSeeking = true;
        seekProgress = progress;
    }

    function handleSeekInput(e: Event) {
        if (audioStore.duration <= 0) return;
        isSeeking = true;
        const input = e.target as HTMLInputElement;
        seekProgress = parseFloat(input.value);
    }

    function handleSeekChange(e: Event) {
        if (audioStore.duration <= 0) return;
        const input = e.target as HTMLInputElement;
        const pct = parseFloat(input.value) / 100;
        audioStore.seek(pct * audioStore.duration);
        isSeeking = false;
    }

    function handleVolume(e: Event) {
        const input = e.target as HTMLInputElement;
        audioStore.setVolume(parseFloat(input.value));
    }

    let trackTitle = $derived.by(() => {
        if (audioStore.currentQueueTrack)
            return audioStore.currentQueueTrack.title;
        if (audioStore.currentTrack !== "None") return audioStore.currentTrack;
        return "Nothing playing";
    });

    let trackArtist = $derived(audioStore.currentQueueTrack?.artist ?? null);

    let isPlaying = $derived(audioStore.playbackState === "Playing");
    let hasTrack = $derived(
        audioStore.currentQueueTrack !== null ||
            audioStore.currentTrack !== "None",
    );
    let pillState = $derived(
        hasTrack ? (isPlaying ? "playing" : "paused") : "idle",
    );
</script>

<div
    class="player-pill-container"
    style="--pill-bg: {settingsStore.glassyPlayerBar
        ? 'rgba(25, 25, 32, 0.35)'
        : 'var(--echo-surface)'};"
>
    <div class="player-pill-wrapper" data-state={pillState}>
        <!-- 1. Fitts's Law Top-Edge Seek Bar -->
        <div class="seek-hitbox group">
            <div class="seek-track">
                <div
                    class="seek-fill pill-accent-bg"
                    style="width: {displayProgress}%"
                ></div>
            </div>
            <div
                class="seek-thumb pill-accent-bg"
                style="left: {displayProgress}%"
            ></div>
            <input
                type="range"
                class="range-overlay"
                min="0"
                max="100"
                step="0.1"
                value={displayProgress}
                onpointerdown={handleSeekPointerDown}
                oninput={handleSeekInput}
                onchange={handleSeekChange}
                aria-label="Seek"
            />
        </div>

        <!-- 2. Main Pill Body -->
        <div class="pill-body" class:is-glass={settingsStore.glassyPlayerBar}>
            <!-- Left Flank: Album Art & Song Details -->
            <div class="flank flank-left">
                <div class="album-art-container">
                    {#if playerArtUrl}
                        <img src={playerArtUrl} alt="Now playing artwork" />
                    {:else}
                        <div
                            class="placeholder"
                            style="background-color: #27272a;"
                        ></div>
                    {/if}
                </div>

                <div class="song-details">
                    <span class="song-title">
                        {trackTitle}
                    </span>
                    <div class="song-time">
                        <span>{formatTime(audioStore.currentTime)}</span>
                        <span class="text-white-20">/</span>
                        <span>{formatTime(audioStore.duration)}</span>
                    </div>
                </div>
            </div>

            <!-- Center: Transport Controls -->
            <div class="center-controls">
                <button
                    class="ctrl-btn"
                    class:active={audioStore.shuffleEnabled}
                    onclick={() => audioStore.toggleShuffle()}
                >
                    <Shuffle size={16} weight="bold" />
                </button>
                <button class="ctrl-btn" onclick={() => audioStore.previous()}>
                    <SkipBack size={20} weight="fill" />
                </button>

                <button
                    class="play-pause-btn pill-accent-bg pill-accent-shadow"
                    onclick={handlePlayPause}
                    disabled={!hasTrack}
                >
                    {#if isPlaying}
                        <Pause size={20} weight="fill" />
                    {:else}
                        <Play size={20} weight="fill" />
                    {/if}
                </button>

                <button class="ctrl-btn" onclick={() => audioStore.next()}>
                    <SkipForward size={20} weight="fill" />
                </button>
                <button
                    class="ctrl-btn"
                    class:active={audioStore.repeatMode !== "Off"}
                    onclick={() => audioStore.cycleRepeat()}
                >
                    {#if audioStore.repeatMode === "One"}
                        <RepeatOnce size={16} weight="bold" />
                    {:else}
                        <Repeat size={16} weight="bold" />
                    {/if}
                </button>
            </div>

            <!-- Right Flank: Utilities -->
            <div class="flank flank-right">
                <button
                    class="ctrl-btn"
                    class:active={fullScreenOpen}
                    onclick={() => (fullScreenOpen = true)}
                    title="Fullscreen Player"
                >
                    <CornersOut size={16} weight="bold" />
                </button>
                <button
                    class="ctrl-btn"
                    class:active={queueOpen}
                    onclick={() => transitionLayout(() => { queueOpen = !queueOpen; })}
                >
                    <ListNumbers size={16} weight="bold" />
                </button>

                <div class="vol-wrapper group">
                    <button
                        class="vol-icon text-muted"
                        onclick={() => audioStore.toggleMute()}
                    >
                        {#if audioStore.isMuted || audioStore.volume === 0}
                            <SpeakerX size={16} weight="bold" />
                        {:else}
                            <SpeakerHigh size={16} weight="bold" />
                        {/if}
                    </button>
                    <div class="vol-hitbox">
                        <div class="vol-track">
                            <div
                                class="vol-fill pill-accent-bg"
                                style="width: {volumeProgress}%"
                            ></div>
                        </div>
                        <div
                            class="vol-thumb pill-accent-bg"
                            style="left: {volumeProgress}%"
                        ></div>
                        <input
                            type="range"
                            class="range-overlay"
                            min="0"
                            max="1"
                            step="0.01"
                            value={audioStore.isMuted ? 0 : audioStore.volume}
                            oninput={handleVolume}
                            aria-label="Volume"
                        />
                    </div>
                </div>
            </div>
        </div>
    </div>
</div>

<style>
    /* CSS Variables matching user HTML */
    :global(body) {
        --text-main: var(--echo-text-1);
        --muted: var(--echo-text-2);
    }

    .text-muted {
        color: var(--muted);
    }
    .text-white-20 {
        color: rgba(255, 255, 255, 0.2);
    }

    .player-pill-container {
        position: fixed;
        bottom: 2rem;
        left: var(--sidebar-w, 80px);
        right: 0px;
        height: 72px;
        z-index: 100;
        display: flex;
        justify-content: center;
        pointer-events: none;
        view-transition-name: player-bar;
    }

    :global(::view-transition-group(player-bar)) {
        z-index: 999;
    }

    .player-pill-wrapper {
        position: relative;
        height: 100%;
        transition:
            width 0.5s cubic-bezier(0.4, 0, 0.2, 1),
            background-color 0.4s ease;
        pointer-events: auto;
    }

    .player-pill-wrapper[data-state="idle"] {
        width: 340px;
    }
    .player-pill-wrapper[data-state="playing"],
    .player-pill-wrapper[data-state="paused"] {
        width: 720px;
    }

    /* Fitts's Law Top-Edge Seek Bar */
    .seek-hitbox {
        position: absolute;
        top: -10px;
        left: 32px;
        right: 32px;
        height: 20px;
        cursor: pointer;
        z-index: 20;
    }
    .seek-track {
        position: absolute;
        top: 50%;
        left: 0;
        width: 100%;
        height: 2px;
        background-color: var(--pill-bg);
        border-top-left-radius: 9999px;
        border-top-right-radius: 9999px;
        transform: translateY(-50%);
        transition: all 0.2s;
        overflow: hidden;
    }
    .seek-hitbox:hover .seek-track {
        height: 4px;
    }
    .seek-fill {
        height: 100%;
        border-top-right-radius: 9999px;
        border-bottom-right-radius: 9999px;
        box-shadow: 0 0 6px rgba(226, 169, 115, 0.2);
    }
    .seek-thumb {
        position: absolute;
        top: 50%;
        transform: translate(-50%, -50%) scale(0.5);
        width: 10px;
        height: 10px;
        border-radius: 50%;
        opacity: 0;
        box-shadow: 0 0 6px rgba(226, 169, 115, 0.3);
        transition:
            opacity 0.2s,
            transform 0.2s;
        pointer-events: none;
    }
    .seek-hitbox:hover .seek-thumb {
        opacity: 1;
        transform: translate(-50%, -50%) scale(1);
    }
    .range-overlay {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        opacity: 0;
        cursor: pointer;
        margin: 0;
    }

    /* Main Pill Body */
    .pill-body {
        width: 100%;
        height: 100%;
        background-color: var(--pill-bg);
        border-radius: 9999px;
        box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
        border: 1px solid rgba(255, 255, 255, 0.05);
        overflow: hidden;
        display: flex;
        align-items: center;
        justify-content: center;
        position: relative;
        z-index: 10;
        backdrop-filter: blur(6px);
        transition: all 0.3s;
    }
    .pill-body.is-glass {
        border: 1px solid rgba(255, 255, 255, 0.13);
        box-shadow:
            0 20px 40px rgba(0, 0, 0, 0.6),
            inset 0 1px 1.5px rgba(255, 255, 255, 0.22),
            inset 0 -1px 1.5px rgba(0, 0, 0, 0.25);
    }
    .pill-body.is-glass::before {
        content: "";
        position: absolute;
        inset: 0;
        border-radius: inherit;
        pointer-events: none;
        backdrop-filter: blur(26px) saturate(1.6) brightness(1.12);
        mask-image: radial-gradient(
            ellipse at center,
            transparent 60%,
            black 100%
        );
        -webkit-mask-image: radial-gradient(
            ellipse at center,
            transparent 60%,
            black 100%
        );
        z-index: -1;
    }

    /* Flanks */
    .flank {
        opacity: 0;
        pointer-events: none;
        transition: opacity 0.3s ease;
        transition-delay: 0s;
    }
    .player-pill-wrapper[data-state="playing"] .flank,
    .player-pill-wrapper[data-state="paused"] .flank {
        opacity: 1;
        pointer-events: auto;
        transition-delay: 0.2s;
    }

    /* Left Flank */
    .flank-left {
        position: absolute;
        left: 24px;
        right: calc(50% + 130px);
        display: flex;
        align-items: center;
        gap: 10px;
        overflow: hidden;
    }
    .album-art-container {
        width: 44px;
        height: 44px;
        flex-shrink: 0;
        border-radius: 12px;
        background-color: #27272a;
        border: 1px solid rgba(255, 255, 255, 0.1);
        overflow: hidden;
        box-shadow:
            0 4px 6px -1px rgba(0, 0, 0, 0.1),
            0 2px 4px -1px rgba(0, 0, 0, 0.06);
    }
    .album-art-container img,
    .album-art-container .placeholder {
        width: 100%;
        height: 100%;
        object-fit: cover;
        background-position: center;
        background-size: cover;
    }
    .song-details {
        display: flex;
        flex-direction: column;
        justify-content: center;
        min-width: 0;
        width: 100%;
        padding-top: 2px;
    }
    .song-title {
        font-size: 15px;
        font-weight: 500;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        display: block;
        line-height: 1.25;
        letter-spacing: 0.025em;
        color: var(--text-main);
        text-shadow: 0 1px 4px rgba(0, 0, 0, 0.8);
    }
    .song-time {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 11px;
        color: var(--muted);
        font-weight: 500;
        margin-top: 2px;
        text-shadow: 0 1px 4px rgba(0, 0, 0, 0.8);
    }

    /* Center Controls */
    .center-controls {
        display: flex;
        align-items: center;
        gap: 2px;
        flex-shrink: 0;
        z-index: 20;
    }
    .ctrl-btn {
        color: var(--muted);
        transition:
            color 0.15s ease,
            background-color 0.15s ease;
        width: 32px;
        height: 32px;
        padding: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 9999px;
        background: transparent;
        border: none;
        cursor: pointer;
        position: relative;
        filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.8));
    }
    .ctrl-btn:hover {
        color: var(--text-main);
        background-color: rgba(255, 255, 255, 0.05);
    }
    .ctrl-btn.active {
        color: var(--text-main);
    }
    .ctrl-btn.active::after {
        content: "";
        position: absolute;
        bottom: 3px;
        left: 50%;
        transform: translateX(-50%);
        width: 4px;
        height: 4px;
        border-radius: 50%;
        background-color: var(--text-main);
    }

    .play-pause-btn {
        width: 44px;
        height: 44px;
        padding: 0;
        border-radius: 9999px;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.3s;
        margin: 0 4px;
        color: var(--echo-void);
        border: none;
        cursor: pointer;
    }
    .play-pause-btn:not(:disabled):hover {
        transform: scale(1.05);
    }
    .play-pause-btn:not(:disabled):active {
        transform: scale(0.95);
    }
    .play-pause-btn:disabled {
        cursor: not-allowed;
    }

    /* Right Flank */
    .flank-right {
        position: absolute;
        right: 24px;
        left: calc(50% + 130px);
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 6px;
        overflow: visible;
    }
    .vol-wrapper {
        display: flex;
        align-items: center;
        gap: 6px;
        cursor: pointer;
    }
    .vol-icon {
        background: transparent;
        border: none;
        padding: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: color 0.15s ease;
        filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.8));
    }
    .vol-wrapper:hover .vol-icon {
        color: var(--text-main);
    }
    .vol-hitbox {
        position: relative;
        width: 48px;
        height: 20px;
        display: flex;
        align-items: center;
        overflow: visible;
    }
    .vol-track {
        position: absolute;
        top: 50%;
        transform: translateY(-50%);
        left: 0;
        width: 100%;
        height: 2px;
        background-color: rgba(255, 255, 255, 0.1);
        border-radius: 9999px;
        overflow: hidden;
    }
    .vol-fill {
        height: 100%;
        transition: background-color 0.4s;
    }
    .vol-thumb {
        position: absolute;
        top: 50%;
        transform: translate(-50%, -50%) scale(0.5);
        width: 8px;
        height: 8px;
        border-radius: 50%;
        opacity: 0;
        box-shadow: 0 0 6px rgba(226, 169, 115, 0.5);
        transition:
            opacity 0.2s,
            transform 0.2s;
        pointer-events: none;
    }
    .vol-wrapper:hover .vol-thumb {
        opacity: 1;
        transform: translate(-50%, -50%) scale(1);
    }

    /* State-based styling */
    /* Base / Idle State Colors */
    .player-pill-wrapper[data-state="idle"] .pill-accent-bg {
        background-color: #4a3c2b;
    } /* Unlit, dull brass */
    .player-pill-wrapper[data-state="idle"] .pill-accent-shadow {
        box-shadow: inset 0 2px 6px rgba(0, 0, 0, 0.6);
    } /* Hardware inset shadow */
    .player-pill-wrapper[data-state="idle"] .play-pause-btn {
        color: #1a140d;
    } /* Very dark, unlit icon */

    /* Accent Colors & System Status Transitions */
    .pill-accent-bg,
    .pill-accent-shadow,
    .album-art-container,
    .song-title {
        transition: all 0.4s ease;
    }

    /* Playing State */
    .player-pill-wrapper[data-state="playing"] .pill-accent-bg {
        background-color: var(--echo-primary);
    }
    .player-pill-wrapper[data-state="playing"] .pill-accent-shadow {
        box-shadow: 0 0 15px rgba(226, 169, 115, 0.3);
    }
    .player-pill-wrapper[data-state="playing"] .song-title {
        color: var(--text-main);
    }
    .player-pill-wrapper[data-state="playing"] .album-art-container {
        filter: grayscale(0%) brightness(1);
    }

    /* Paused State */
    .player-pill-wrapper[data-state="paused"] .pill-accent-bg {
        background-color: var(--echo-primary-dark);
    }
    .player-pill-wrapper[data-state="paused"] .pill-accent-shadow {
        box-shadow: 0 0 10px rgba(181, 142, 98, 0.15);
    }
    .player-pill-wrapper[data-state="paused"] .song-title {
        color: var(--muted);
    }
    .player-pill-wrapper[data-state="paused"] .album-art-container {
        filter: grayscale(40%) brightness(0.6);
    }
</style>
