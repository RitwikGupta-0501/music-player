import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface PlayerSyncPayload {
    state: string;
    position: number;
    duration: number;
    track: string;
}

export class AudioStore {
    playbackState = $state("Stopped");
    currentTrack = $state("None");
    currentTime = $state(0);
    duration = $state(0);
    
    // Interpolation anchor points
    private _syncPosition = 0;
    private _syncTimestamp = 0;
    private _isPlaying = false;
    private _rafId: number | null = null;
    
    private unlistenSync: UnlistenFn | null = null;

    async init() {
        this.unlistenSync = await listen<PlayerSyncPayload>("player-sync", (e) => {
            const payload = e.payload;
            
            // Anchor the interpolation clock to the backend's reported position
            this._syncPosition = payload.position;
            this._syncTimestamp = performance.now();
            this._isPlaying = payload.state === "Playing";
            
            // Update reactive state
            this.playbackState = payload.state;
            this.duration = payload.duration;
            this.currentTrack = payload.track || "None";
            this.currentTime = payload.position;
            
            // Reset on stop
            if (payload.state === "Stopped") {
                this.currentTime = 0;
                this.duration = 0;
            }
        });

        // Start the visual clock
        this.startClock();
    }

    destroy() {
        if (this.unlistenSync) this.unlistenSync();
        this.stopClock();
    }

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

    async load(path: string) {
        await invoke("load_audio", { path });
    }

    async play() {
        await invoke("play_audio");
    }

    async pause() {
        await invoke("pause_audio");
    }

    async stop() {
        await invoke("stop_audio");
    }
}

export const audioStore = new AudioStore();
