<script lang="ts">
    import { audioStore } from "$lib/stores/audio.svelte";
    import { 
        Play, 
        Pause, 
        SkipBack, 
        SkipForward, 
        Shuffle, 
        Repeat, 
        Repeat1, 
        Volume2, 
        VolumeX 
    } from "lucide-svelte";

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

    // Display-friendly track name
    let displayTrack = $derived(() => {
        if (!audioStore.currentQueueTrack) return audioStore.currentTrack !== "None" ? audioStore.currentTrack : "Ready to play";
        const t = audioStore.currentQueueTrack;
        return t.artist ? `${t.title} — ${t.artist}` : t.title;
    });
</script>

<div class="bottom-player">
    <!-- Left: Track Info -->
    <div class="player-info">
        <span class="now-playing-title">{displayTrack()}</span>
        {#if audioStore.queue.length > 0}
            <span class="text-muted">Track {audioStore.queueIndex + 1} of {audioStore.queue.length}</span>
        {/if}
    </div>

    <!-- Center: Controls + Progress Bar -->
    <div class="player-center">
        <!-- Transport Controls -->
        <div class="player-controls">
            <button
                class="control-btn"
                class:active={audioStore.shuffleEnabled}
                onclick={() => audioStore.toggleShuffle()}
                title="Shuffle"
            >
                <Shuffle size={16} />
            </button>

            <button class="control-btn" onclick={() => audioStore.previous()} title="Previous">
                <SkipBack size={18} fill="currentColor" />
            </button>

            <button class="control-btn play-btn" onclick={handlePlayPause} title={audioStore.playbackState === "Playing" ? "Pause" : "Play"}>
                {#if audioStore.playbackState === "Playing"}
                    <Pause size={20} fill="currentColor" />
                {:else}
                    <Play size={20} fill="currentColor" />
                {/if}
            </button>

            <button class="control-btn" onclick={() => audioStore.next()} title="Next">
                <SkipForward size={18} fill="currentColor" />
            </button>

            <button
                class="control-btn"
                class:active={audioStore.repeatMode !== 'off'}
                onclick={() => audioStore.cycleRepeat()}
                title="Repeat: {audioStore.repeatMode}"
            >
                {#if audioStore.repeatMode === 'one'}
                    <Repeat1 size={16} />
                {:else}
                    <Repeat size={16} />
                {/if}
            </button>
        </div>

        <!-- Progress Section -->
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
    </div>

    <!-- Right: Volume controls -->
    <div class="player-right">
        <button class="control-btn volume-btn" onclick={() => audioStore.toggleMute()} title={audioStore.isMuted ? "Unmute" : "Mute"}>
            {#if audioStore.isMuted || audioStore.volume === 0}
                <VolumeX size={18} />
            {:else}
                <Volume2 size={18} />
            {/if}
        </button>
        <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={audioStore.isMuted ? 0 : audioStore.volume}
            oninput={(e) => audioStore.setVolume(parseFloat(e.currentTarget.value))}
            class="volume-slider"
        />
    </div>
</div>

<style>
    /* ── Progress Bar Theme Tokens ── */
    .progress-track {
        --pb-height: 4px;
        --pb-thumb-size: 10px;
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
        width: 30%;
        min-width: 200px;
    }

    .player-center {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.5rem;
        max-width: 600px;
    }

    .progress-section {
        width: 100%;
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }

    .time {
        font-family: var(--font-body);
        font-size: 0.8rem;
        color: var(--color-chalk-muted);
        min-width: 35px;
        text-align: center;
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
        box-shadow: 0 0 8px var(--pb-thumb-glow);
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
        gap: 1rem;
    }

    .control-btn {
        background: transparent;
        border: none;
        color: var(--color-chalk-muted);
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 0.4rem;
        border-radius: 50%;
        cursor: pointer;
        transition: all 0.15s ease;
    }

    .control-btn:hover {
        color: #fff;
        background: rgba(255, 255, 255, 0.08);
    }

    .control-btn.active {
        color: var(--color-cyan);
    }

    .control-btn.play-btn {
        background: var(--color-cyan);
        color: var(--color-bg);
        padding: 0.6rem;
    }

    .control-btn.play-btn:hover {
        background: #fff;
        color: var(--color-bg);
        transform: scale(1.05);
    }

    /* ── Player Right (Volume) ── */
    .player-right {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        justify-content: flex-end;
        width: 30%;
        min-width: 150px;
    }

    .volume-slider {
        -webkit-appearance: none;
        appearance: none;
        width: 100px;
        height: 4px;
        border-radius: 2px;
        background: rgba(255, 255, 255, 0.15);
        outline: none;
        cursor: pointer;
        transition: background 0.15s ease;
    }

    .volume-slider:hover {
        background: rgba(255, 255, 255, 0.25);
    }

    .volume-slider::-webkit-slider-thumb {
        -webkit-appearance: none;
        appearance: none;
        width: 10px;
        height: 10px;
        border-radius: 50%;
        background: var(--color-cyan);
        box-shadow: 0 0 8px rgba(102, 252, 241, 0.5);
        cursor: pointer;
        transition: transform 0.1s ease;
    }

    .volume-slider::-webkit-slider-thumb:hover {
        transform: scale(1.2);
    }
</style>
