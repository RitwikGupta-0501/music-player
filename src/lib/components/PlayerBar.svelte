<script lang="ts">
    import { audioStore } from "$lib/stores/audio.svelte";

    function handlePlayPause() {
        if (audioStore.playbackState === "Playing") {
            audioStore.pause();
        } else {
            audioStore.play();
        }
    }

    function formatTime(seconds: number) {
        const mins = Math.floor(seconds / 60);
        const secs = Math.floor(seconds % 60);
        return `${mins}:${secs.toString().padStart(2, '0')}`;
    }

    function handleSeek(e: MouseEvent) {
        if (audioStore.duration <= 0) return;
        const track = e.currentTarget as HTMLElement;
        const rect = track.getBoundingClientRect();
        const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
        audioStore.seek(pct * audioStore.duration);
    }

    let progress = $derived(audioStore.duration > 0 ? (audioStore.currentTime / audioStore.duration) * 100 : 0);

    // Display-friendly track name (strip path, show just filename)
    let displayTrack = $derived(() => {
        if (!audioStore.currentQueueTrack) return audioStore.currentTrack !== "None" ? audioStore.currentTrack : "Ready to play";
        const t = audioStore.currentQueueTrack;
        return t.artist ? `${t.title} — ${t.artist}` : t.title;
    });

    function repeatLabel(): string {
        if (audioStore.repeatMode === 'one') return '🔂';
        if (audioStore.repeatMode === 'all') return '🔁';
        return '🔁';
    }
</script>

<div class="bottom-player">
    <!-- Track Info -->
    <div class="player-info">
        <span class="now-playing-title">{displayTrack()}</span>
        {#if audioStore.queue.length > 0}
            <span class="text-muted">Track {audioStore.queueIndex + 1} of {audioStore.queue.length}</span>
        {/if}
    </div>

    <!-- Progress Bar -->
    <div class="progress-section">
        <span class="time">{formatTime(audioStore.currentTime)}</span>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="progress-track" style="--progress: {progress}%" onclick={handleSeek}>
            <div class="progress-fill"></div>
            <div class="progress-thumb"></div>
        </div>
        <span class="time">{formatTime(audioStore.duration)}</span>
    </div>

    <!-- Transport Controls -->
    <div class="player-controls">
        <button
            class="control-btn"
            class:active={audioStore.shuffleEnabled}
            onclick={() => audioStore.toggleShuffle()}
            title="Shuffle"
        >🔀</button>

        <button class="control-btn" onclick={() => audioStore.previous()} title="Previous">⏮</button>

        <button class="control-btn play-btn" onclick={handlePlayPause} title={audioStore.playbackState === "Playing" ? "Pause" : "Play"}>
            {audioStore.playbackState === "Playing" ? "⏸" : "▶"}
        </button>

        <button class="control-btn" onclick={() => audioStore.next()} title="Next">⏭</button>

        <button
            class="control-btn"
            class:active={audioStore.repeatMode !== 'off'}
            onclick={() => audioStore.cycleRepeat()}
            title="Repeat: {audioStore.repeatMode}"
        >{repeatLabel()}</button>
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

    .now-playing-title {
        font-family: var(--font-display);
        font-weight: 600;
        color: #fff;
        font-size: 0.95rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        max-width: 280px;
        display: block;
    }

    .player-info {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        min-width: 200px;
    }

    .progress-section {
        flex: 1;
        display: flex;
        align-items: center;
        gap: 1rem;
        margin: 0 2rem;
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

    /* ── Transport Controls ── */
    .player-controls {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .control-btn {
        background: transparent;
        border: none;
        color: var(--color-chalk-muted);
        font-size: 1.2rem;
        padding: 0.4rem 0.6rem;
        border-radius: 6px;
        cursor: pointer;
        transition: all 0.15s ease;
        line-height: 1;
    }

    .control-btn:hover {
        color: #fff;
        background: rgba(255, 255, 255, 0.08);
        transform: none;
    }

    .control-btn.active {
        color: var(--color-cyan);
    }

    .control-btn.play-btn {
        font-size: 1.6rem;
        padding: 0.4rem 0.8rem;
        color: var(--color-cyan);
    }
</style>
