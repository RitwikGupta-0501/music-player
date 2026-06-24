import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface PlayerSyncPayload {
    state: string;
    position: number;
    duration: number;
    track: string;
}

export interface QueueTrack {
    id: number;
    title: string;
    artist: string | null;
    file_path: string;
}

export class AudioStore {
    // ── Playback State ──
    playbackState = $state("Stopped");
    currentTrack = $state("None");
    currentTime = $state(0);
    duration = $state(0);

    // ── Queue State ──
    queue = $state<QueueTrack[]>([]);
    queueIndex = $state(-1);

    // ── Shuffle & Repeat ──
    shuffleEnabled = $state(false);
    repeatMode = $state<'off' | 'all' | 'one'>('off');

    // ── Volume & Mute ──
    volume = $state(1.0);
    isMuted = $state(false);

    // Internal: shuffle order maps visual index → actual queue index
    private _shuffleOrder: number[] = [];
    // Internal: history of played indices for "previous" while shuffled
    private _shuffleHistory: number[] = [];

    // ── Interpolation internals ──
    private _syncPosition = 0;
    private _syncTimestamp = 0;
    private _isPlaying = false;
    private _rafId: number | null = null;
    private _autoAdvancing = false;
    
    private unlistenSync: UnlistenFn | null = null;
    private unlistenTrackEnded: UnlistenFn | null = null;
    private _volumeSaveTimer: ReturnType<typeof setTimeout> | null = null;

    // ── Derived ──
    currentQueueTrack = $derived(
        this.queueIndex >= 0 && this.queueIndex < this.queue.length
            ? this.queue[this.queueIndex]
            : null
    );

    // ══════════════════════════════════════════
    //  LIFECYCLE
    // ══════════════════════════════════════════

    async init() {
        this.unlistenSync = await listen<PlayerSyncPayload>("player-sync", (e) => {
            const payload = e.payload;
            
            this._syncPosition = payload.position;
            this._syncTimestamp = performance.now();
            this._isPlaying = payload.state === "Playing";
            
            this.playbackState = payload.state;
            this.duration = payload.duration;
            this.currentTrack = payload.track || "None";
            this.currentTime = payload.position;
            
            if (payload.state === "Stopped") {
                this.currentTime = 0;
                this.duration = 0;
            }
        });

        this.unlistenTrackEnded = await listen("track-ended", () => {
            this._autoAdvancing = true;
            this.next();
        });

        this.startClock();

        // Load volume and mute settings from DB
        try {
            const persistedVolume = await invoke<string | null>("get_setting", { key: "volume" });
            if (persistedVolume !== null) {
                this.volume = parseFloat(persistedVolume);
                await invoke("set_volume", { volume: this.volume });
            }
            const persistedMute = await invoke<string | null>("get_setting", { key: "mute" });
            if (persistedMute !== null) {
                this.isMuted = persistedMute === "true";
                await invoke("set_mute", { mute: this.isMuted });
            }
        } catch (e) {
            console.error("Failed to load settings:", e);
        }
    }

    destroy() {
        if (this.unlistenSync) this.unlistenSync();
        if (this.unlistenTrackEnded) this.unlistenTrackEnded();
        this.stopClock();
    }

    // ══════════════════════════════════════════
    //  INTERPOLATION CLOCK
    // ══════════════════════════════════════════

    /** 60fps visual clock — runs locally, zero IPC */
    private tick = () => {
        if (this._isPlaying && this.duration > 0) {
            const elapsed = (performance.now() - this._syncTimestamp) / 1000;
            this.currentTime = Math.min(this._syncPosition + elapsed, this.duration);
        }
        this._rafId = requestAnimationFrame(this.tick);
    };

    private startClock() {
        if (this._rafId === null) {
            this._rafId = requestAnimationFrame(this.tick);
        }
    }

    private stopClock() {
        if (this._rafId !== null) {
            cancelAnimationFrame(this._rafId);
            this._rafId = null;
        }
    }

    // ══════════════════════════════════════════
    //  BASIC TRANSPORT
    // ══════════════════════════════════════════

    async load(path: string) {
        this._autoAdvancing = false;
        this._syncPosition = 0;
        this._syncTimestamp = performance.now();
        try {
            await invoke("load_audio", { path });
        } catch (e) {
            if (e === "FILE_NOT_FOUND") {
                // Prune the dead track from the DB (fire and forget).
                invoke("remove_track_by_path", { path }).catch(() => {});
                // Remove from the in-memory queue. Since queueIndex was already
                // set to point at this track before load() was called, removing it
                // shifts the successor into queueIndex — try loading that next.
                this.queue = this.queue.filter(t => t.file_path !== path);
                if (this.queueIndex < this.queue.length) {
                    await this.load(this.queue[this.queueIndex].file_path);
                } else {
                    this.queueIndex = -1;
                    await this.stop();
                }
            }
        }
    }

    async play() {
        await invoke("play_audio");
    }

    async pause() {
        await invoke("pause_audio");
    }

    async stop() {
        this.queueIndex = -1; // Reset queue visual state so PlayerBar goes Idle
        await invoke("stop_audio");
    }

    // ══════════════════════════════════════════
    //  SEEK
    // ══════════════════════════════════════════

    async seek(position: number) {
        // Clamp to valid range
        position = Math.max(0, Math.min(position, this.duration));
        
        // Immediately snap the interpolation anchor so UI feels instant
        this._syncPosition = position;
        this._syncTimestamp = performance.now();
        this.currentTime = position;
        
        await invoke("seek_audio", { position });
    }

    // ══════════════════════════════════════════
    //  QUEUE MANAGEMENT
    // ══════════════════════════════════════════

    async jumpToIndex(index: number) {
        if (index < 0 || index >= this.queue.length) return;
        this.queueIndex = index;
        this._shuffleHistory.push(index);
        this._autoAdvancing = false;
        await this.load(this.queue[index].file_path);
    }

    /** Replace the queue and start playing at startIndex */
    async setQueue(tracks: QueueTrack[], startIndex: number = 0) {
        this.queue = tracks;
        this.queueIndex = startIndex;
        this._shuffleHistory = [startIndex];
        
        // If shuffle is on, regenerate the order
        if (this.shuffleEnabled) {
            this.generateShuffleOrder();
        }

        if (tracks.length > 0 && startIndex < tracks.length) {
            await this.load(tracks[startIndex].file_path);
        }
    }

    /** Append a track to the end of the queue */
    addToQueue(track: QueueTrack) {
        this.queue = [...this.queue, track];
        // If shuffle is on, add the new index to the shuffle order
        if (this.shuffleEnabled) {
            this._shuffleOrder.push(this.queue.length - 1);
        }
    }

    /** Clear the queue and stop */
    async clearQueue() {
        this.queue = [];
        this.queueIndex = -1;
        this._shuffleOrder = [];
        this._shuffleHistory = [];
        await this.stop();
    }

    // ══════════════════════════════════════════
    //  SKIP: NEXT / PREVIOUS
    // ══════════════════════════════════════════

    async next() {
        if (this.queue.length === 0) return;

        // Repeat One: re-seek to 0 (but only on auto-advance, not manual skip)
        // Manual next() should always advance, so we check _autoAdvancing
        if (this.repeatMode === 'one' && this._autoAdvancing) {
            this._autoAdvancing = false;
            await this.seek(0);
            await this.play();
            return;
        }
        this._autoAdvancing = false;

        let nextIndex: number;

        if (this.shuffleEnabled) {
            // Find current position in shuffle order and advance
            const shufflePos = this._shuffleOrder.indexOf(this.queueIndex);
            const nextShufflePos = shufflePos + 1;

            if (nextShufflePos >= this._shuffleOrder.length) {
                // End of shuffle order
                if (this.repeatMode === 'all') {
                    this.generateShuffleOrder(); // reshuffle for next cycle
                    nextIndex = this._shuffleOrder[0];
                } else {
                    await this.stop();
                    return;
                }
            } else {
                nextIndex = this._shuffleOrder[nextShufflePos];
            }
        } else {
            nextIndex = this.queueIndex + 1;
            if (nextIndex >= this.queue.length) {
                if (this.repeatMode === 'all') {
                    nextIndex = 0;
                } else {
                    await this.stop();
                    return;
                }
            }
        }

        this._shuffleHistory.push(nextIndex);
        this.queueIndex = nextIndex;
        await this.load(this.queue[nextIndex].file_path);
    }

    async previous() {
        if (this.queue.length === 0) return;

        // If more than 3 seconds in, restart the current track
        if (this.currentTime > 3) {
            await this.seek(0);
            return;
        }

        if (this.shuffleEnabled && this._shuffleHistory.length > 1) {
            // Pop current, go to previous in history
            this._shuffleHistory.pop();
            const prevIndex = this._shuffleHistory[this._shuffleHistory.length - 1];
            this.queueIndex = prevIndex;
            await this.load(this.queue[prevIndex].file_path);
        } else if (!this.shuffleEnabled && this.queueIndex > 0) {
            this.queueIndex--;
            await this.load(this.queue[this.queueIndex].file_path);
        } else {
            // At the start of the queue, just restart
            await this.seek(0);
        }
    }

    // ══════════════════════════════════════════
    //  SHUFFLE
    // ══════════════════════════════════════════

    toggleShuffle() {
        this.shuffleEnabled = !this.shuffleEnabled;
        if (this.shuffleEnabled) {
            this.generateShuffleOrder();
        } else {
            this._shuffleOrder = [];
        }
    }

    /** Fisher-Yates shuffle. Current track goes to position 0 so it stays "now playing". */
    private generateShuffleOrder() {
        const indices = Array.from({ length: this.queue.length }, (_, i) => i);
        
        // Remove current index from the pool
        const currentIdx = this.queueIndex >= 0 ? this.queueIndex : 0;
        const filtered = indices.filter(i => i !== currentIdx);
        
        // Fisher-Yates
        for (let i = filtered.length - 1; i > 0; i--) {
            const j = Math.floor(Math.random() * (i + 1));
            [filtered[i], filtered[j]] = [filtered[j], filtered[i]];
        }
        
        // Current track goes first
        this._shuffleOrder = [currentIdx, ...filtered];
    }

    // ══════════════════════════════════════════
    //  REPEAT
    // ══════════════════════════════════════════

    cycleRepeat() {
        if (this.repeatMode === 'off') this.repeatMode = 'all';
        else if (this.repeatMode === 'all') this.repeatMode = 'one';
        else this.repeatMode = 'off';
    }

    // ══════════════════════════════════════════
    //  VOLUME CONTROL
    // ══════════════════════════════════════════

    setVolume(volume: number) {
        this.volume = Math.max(0, Math.min(volume, 1));
        invoke("set_volume", { volume: this.volume }); // fire-and-forget, no await
        if (this._volumeSaveTimer) clearTimeout(this._volumeSaveTimer);
        this._volumeSaveTimer = setTimeout(() => {
            invoke("set_setting", { key: "volume", value: this.volume.toString() }).catch(console.error);
        }, 500);
    }

    async toggleMute() {
        this.isMuted = !this.isMuted;
        await invoke("set_mute", { mute: this.isMuted });
        try {
            await invoke("set_setting", { key: "mute", value: this.isMuted.toString() });
        } catch (e) {
            console.error(e);
        }
    }
}

export const audioStore = new AudioStore();
