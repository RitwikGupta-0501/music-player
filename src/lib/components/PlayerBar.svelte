<script lang="ts">
    import { audioStore } from "$lib/stores/audio.svelte";

    function handlePlay() {
        audioStore.play();
    }

    function handlePause() {
        audioStore.pause();
    }

    function handleStop() {
        audioStore.stop();
    }

    function formatTime(seconds: number) {
        const mins = Math.floor(seconds / 60);
        const secs = Math.floor(seconds % 60);
        return `${mins}:${secs.toString().padStart(2, '0')}`;
    }

    let progress = $derived(audioStore.duration > 0 ? (audioStore.currentTime / audioStore.duration) * 100 : 0);
</script>

<div class="bottom-player">
    <div class="player-info">
        <h3 class="text-cyan">{audioStore.playbackState}</h3>
        <span class="text-muted" style="max-width: 300px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
            {audioStore.currentTrack !== "None" ? audioStore.currentTrack : "Ready to play"}
        </span>
    </div>

    <div class="progress-section">
        <span class="time">{formatTime(audioStore.currentTime)}</span>
        <div class="progress-track" style="--progress: {progress}%">
            <div class="progress-fill"></div>
            <div class="progress-thumb"></div>
        </div>
        <span class="time">{formatTime(audioStore.duration)}</span>
    </div>

    <div class="player-controls">
        <button onclick={handlePlay}>Play</button>
        <button onclick={handlePause}>Pause</button>
        <button onclick={handleStop} style="border-color: var(--color-danger); color: var(--color-danger);">Stop</button>
    </div>
</div>

<style>
    /* ── Progress Bar Theme Tokens ── */
    .progress-track {
        --pb-height: 4px;
        --pb-thumb-size: 12px;
        --pb-fill-color: var(--color-cyan);
        --pb-track-color: rgba(255, 255, 255, 0.1);
        --pb-thumb-glow: rgba(102, 252, 241, 0.5);
        --pb-radius: 2px;
    }

    .progress-section {
        flex: 1;
        display: flex;
        align-items: center;
        gap: 1rem;
        margin: 0 3rem;
    }

    .time {
        font-family: var(--font-body);
        font-size: 0.8rem;
        color: var(--color-chalk-muted);
        min-width: 35px;
    }

    .progress-track {
        position: relative;
        flex: 1;
        height: var(--pb-height);
        background: var(--pb-track-color);
        border-radius: var(--pb-radius);
        cursor: pointer;
    }

    .progress-fill {
        position: absolute;
        top: 0;
        left: 0;
        height: 100%;
        width: var(--progress);
        background: var(--pb-fill-color);
        border-radius: var(--pb-radius);
        transition: width 0.05s linear;
    }

    .progress-thumb {
        position: absolute;
        top: 50%;
        left: var(--progress);
        width: var(--pb-thumb-size);
        height: var(--pb-thumb-size);
        background: var(--pb-fill-color);
        border-radius: 50%;
        transform: translate(-50%, -50%);
        box-shadow: 0 0 10px var(--pb-thumb-glow);
        opacity: 0;
        transition: opacity 0.2s ease;
    }

    .progress-track:hover .progress-thumb {
        opacity: 1;
    }
</style>

