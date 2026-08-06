import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface PlayerSyncPayload {
    state: string;
    position: number;
    duration: number;
    track: string;
}

interface QueueChangePayload {
    tracks: QueueTrack[];
    current_position: number;
    current_track: QueueTrack | null;
    repeat_mode: string;
    queue_mode: string;
}

interface TrackSource {
    type: 'Local' | 'Remote';
    // Local fields
    track_id?: number;
    file_path?: string;
    album_id?: number | null;
    // Remote fields
    provider_id?: string;
    remote_track_id?: string;
    stream_url?: string | null;
    quality_hint?: string | null;
    cover_art_url?: string | null;
    duration_ms?: number | null;
}

export interface QueueTrack {
    instanceId: string;
    title: string;
    artist: string | null;
    trackNumber?: number | null;
    source: TrackSource;
}

export class AudioStore {
    // ══════════════════════════════════════════
    // PLAYBACK STATE (NOT cached from backend)
    // ══════════════════════════════════════════

    playbackState = $state("Stopped");
    currentTrack = $state("None");
    currentTime = $state(0);
    duration = $state(0);
    volume = $state(1.0);
    isMuted = $state(false);

    trackClickBehavior = $state<"interrupt" | "clear" | "append">("interrupt");

    // ══════════════════════════════════════════
    // QUEUE STATE (Cached from backend events)
    // ══════════════════════════════════════════

    queue = $state<QueueTrack[]>([]);
    currentPosition = $state(0);
    currentQueueId = $state<string | null>(null);
    repeatMode = $state("Off");
    shuffleEnabled = $state(false);

    // ══════════════════════════════════════════
    // INTERNAL: Interpolation for smooth playback
    // ══════════════════════════════════════════

    private _syncPosition = 0;
    private _syncTimestamp = 0;
    private _isPlaying = false;
    private _rafId: number | null = null;
    private _autoAdvancing = false;

    private unlistenSync: UnlistenFn | null = null;
    private unlistenTrackEnded: UnlistenFn | null = null;
    private unlistenQueueChanged: UnlistenFn | null = null;
    private _volumeSaveTimer: ReturnType<typeof setTimeout> | null = null;

    // ══════════════════════════════════════════
    // DERIVED STATE
    // ══════════════════════════════════════════

    currentQueueTrack = $derived(
        this.currentQueueId
            ? this.queue.find((t) => t.instanceId === this.currentQueueId) || null
            : null
    );

    // ══════════════════════════════════════════
    // LIFECYCLE
    // ══════════════════════════════════════════

    async init() {
        // Listen to playback sync events (backend → frontend)
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

        // Listen to queue changes from backend
        this.unlistenQueueChanged = await listen<QueueChangePayload>(
            "queue-changed",
            (e) => {
                const payload = e.payload;
                this.queue = payload.tracks;
                this.currentPosition = payload.current_position;
                this.currentQueueId = payload.current_track?.instanceId || null;
                this.repeatMode = payload.repeat_mode;
                this.shuffleEnabled = payload.queue_mode === "Shuffle";
            }
        );

        // Track end detection
        this.unlistenTrackEnded = await listen("track-ended", () => {
            this._autoAdvancing = true;
            this.skipForward(1);
        });

        // Load persisted settings
        try {
            const persistedVolume = await invoke<string | null>("get_setting", {
                key: "volume",
            });
            if (persistedVolume !== null) {
                this.volume = parseFloat(persistedVolume);
                await invoke("set_volume", { volume: this.volume });
            }
            const persistedMute = await invoke<string | null>("get_setting", {
                key: "mute",
            });
            if (persistedMute !== null) {
                this.isMuted = persistedMute === "true";
                await invoke("set_mute", { mute: this.isMuted });
            }
            const persistedClickBehavior = await invoke<string | null>("get_setting", {
                key: "track_click_behavior",
            });
            if (persistedClickBehavior !== null) {
                this.trackClickBehavior = persistedClickBehavior as "interrupt" | "clear" | "append";
            }
        } catch (e) {
            console.error("Failed to load settings:", e);
        }

        try {
            await invoke("sync_playback_state");
        } catch (e) {
            console.error("Failed to sync playback state on boot:", e);
        }

        this.startClock();
    }

    destroy() {
        if (this.unlistenSync) this.unlistenSync();
        if (this.unlistenTrackEnded) this.unlistenTrackEnded();
        if (this.unlistenQueueChanged) this.unlistenQueueChanged();
        this.stopClock();
        if (this._volumeSaveTimer) clearTimeout(this._volumeSaveTimer);
    }

    // ══════════════════════════════════════════
    // PLAYBACK CLOCK (60fps interpolation)
    // ══════════════════════════════════════════

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
    // PLAYBACK COMMANDS
    // ══════════════════════════════════════════

    async play() {
        try {
            await invoke("play_audio");
        } catch (e) {
            console.error("Play failed:", e);
        }
    }

    async pause() {
        try {
            await invoke("pause_audio");
        } catch (e) {
            console.error("Pause failed:", e);
        }
    }

    async stop() {
        try {
            await invoke("stop_audio");
        } catch (e) {
            console.error("Stop failed:", e);
        }
    }

    async seek(position: number) {
        position = Math.max(0, Math.min(position, this.duration));

        this._syncPosition = position;
        this._syncTimestamp = performance.now();
        this.currentTime = position;

        try {
            await invoke("seek_audio", { position });
        } catch (e) {
            console.error("Seek failed:", e);
        }
    }

    // ══════════════════════════════════════════
    // VOLUME COMMANDS
    // ══════════════════════════════════════════

    async setVolume(volume: number) {
        this.volume = Math.max(0, Math.min(1, volume));

        if (this._volumeSaveTimer) clearTimeout(this._volumeSaveTimer);
        this._volumeSaveTimer = setTimeout(async () => {
            try {
                await invoke("set_setting", {
                    key: "volume",
                    value: this.volume.toString(),
                });
            } catch (e) {
                console.error("Failed to save volume:", e);
            }
        }, 500);

        try {
            await invoke("set_volume", { volume: this.volume });
        } catch (e) {
            console.error("Set volume failed:", e);
        }
    }

    async setMute(mute: boolean) {
        this.isMuted = mute;

        try {
            await invoke("set_setting", { key: "mute", value: mute.toString() });
            await invoke("set_mute", { mute });
        } catch (e) {
            console.error("Set mute failed:", e);
        }
    }

    async setTrackClickBehavior(behavior: "interrupt" | "clear" | "append") {
        this.trackClickBehavior = behavior;
        try {
            await invoke("set_setting", { key: "track_click_behavior", value: behavior });
        } catch (e) {
            console.error("Failed to save track click behavior:", e);
        }
    }

    // ══════════════════════════════════════════
    // QUEUE COMMANDS (All via backend)
    // ══════════════════════════════════════════

    private formatQueueTrack(t: any): QueueTrack {
        const instanceId = crypto.randomUUID();
        const isRemote = !!(t.stream_url);
        const source: TrackSource = isRemote
            ? {
                type: 'Remote',
                provider_id: t.provider_id ?? 'unknown',
                stream_url: t.stream_url,
                quality_hint: t.quality_hint ?? null,
                cover_art_url: t.cover_art_url ?? null,
                duration_ms: t.duration_ms ?? null,
            }
            : {
                type: 'Local',
                track_id: t.id ?? t.track_id ?? t.trackId ?? -1,
                file_path: t.file_path ?? t.filePath ?? '',
                album_id: t.album_id ?? t.albumId ?? null,
            };

        return {
            instanceId,
            title: t.title,
            artist: t.artist ?? null,
            trackNumber: t.track_number ?? t.trackNumber ?? null,
            source,
        };
    }

    async setQueue(tracks: any[], startIndex: number = 0) {
        try {
            const tracksWithIds = tracks.map((t) => this.formatQueueTrack(t));
            await invoke("set_queue", { tracks: tracksWithIds, startIndex });

            // Load the first track immediately
            if (tracksWithIds.length > startIndex) {
                const t = tracksWithIds[startIndex];
                await invoke("load_audio", {
                    source: t.source,
                    title: t.title,
                    artist: t.artist || null,
                    album: null
                });
            }
        } catch (e) {
            console.error("Set queue failed:", e);
        }
    }

    async addToQueue(track: any) {
        try {
            const trackWithId = this.formatQueueTrack(track);
            await invoke("add_to_queue", { track: trackWithId });
        } catch (e) {
            console.error("Add to queue failed:", e);
        }
    }

    async playNext(track: any) {
        try {
            const trackWithId = this.formatQueueTrack(track);
            const event = await invoke<QueueChangePayload>("add_to_queue", { track: trackWithId });
            
            if (event && event.tracks && event.tracks.length > 1) {
                const fromIndex = event.tracks.length - 1;
                const toIndex = Math.min(event.current_position + 1, event.tracks.length - 1);
                
                if (fromIndex !== toIndex) {
                    await invoke("reorder_queue", { fromIndex, toIndex });
                }
            }
        } catch (e) {
            console.error("Play next failed:", e);
        }
    }

    async playInterrupt(track: any) {
        try {
            const trackWithId = this.formatQueueTrack(track);
            const event = await invoke<QueueChangePayload>("add_to_queue", { track: trackWithId });
            
            if (event && event.tracks && event.tracks.length > 1) {
                const fromIndex = event.tracks.length - 1;
                const toIndex = Math.min(event.current_position + 1, event.tracks.length - 1);
                
                if (fromIndex !== toIndex) {
                    await invoke("reorder_queue", { fromIndex, toIndex });
                }

                await this.jumpToTrack(trackWithId.instanceId);
            }
        } catch (e) {
            console.error("Play interrupt failed:", e);
        }
    }

    async clearQueue() {
        try {
            await invoke("clear_queue");
        } catch (e) {
            console.error("Clear queue failed:", e);
        }
    }

    async skipForward(count: number = 1) {
        try {
            const event = await invoke<QueueChangePayload>("skip_forward", { count });
            if (event.current_track) {
                const t = event.current_track;
                await invoke("load_audio", {
                    source: t.source,
                    title: t.title,
                    artist: t.artist || null,
                    album: null
                });
            }
        } catch (e) {
            console.error("Skip forward failed:", e);
        }
    }

    async skipBackward(count: number = 1) {
        try {
            const event = await invoke<QueueChangePayload>("skip_backward", { count });
            if (event.current_track) {
                const t = event.current_track;
                await invoke("load_audio", {
                    source: t.source,
                    title: t.title,
                    artist: t.artist || null,
                    album: null
                });
            }
        } catch (e) {
            console.error("Skip backward failed:", e);
        }
    }

    async jumpToTrack(instanceId: string) {
        try {
            const event = await invoke<QueueChangePayload>("jump_to_track", {
                instanceId,
            });
            if (event.current_track) {
                const t = event.current_track;
                await invoke("load_audio", {
                    source: t.source,
                    title: t.title,
                    artist: t.artist || null,
                    album: null
                });
            }
        } catch (e) {
            console.error("Jump to track failed:", e);
        }
    }

    async reorderQueue(fromIndex: number, toIndex: number) {
        try {
            await invoke("reorder_queue", {
                fromIndex,
                toIndex,
            });
        } catch (e) {
            console.error("Reorder queue failed:", e);
        }
    }

    async setRepeatMode(mode: "Off" | "All" | "One") {
        try {
            await invoke("set_repeat_mode", { mode });
        } catch (e) {
            console.error("Set repeat mode failed:", e);
        }
    }

    async setShuffle(enabled: boolean) {
        try {
            await invoke("set_shuffle", { enabled });
        } catch (e) {
            console.error("Set shuffle failed:", e);
        }
    }

    // ══════════════════════════════════════════
    // HELPER METHODS (for UI convenience)
    // ══════════════════════════════════════════

    async toggleShuffle() {
        await this.setShuffle(!this.shuffleEnabled);
    }

    async toggleMute() {
        await this.setMute(!this.isMuted);
    }

    async cycleRepeat() {
        const modes: ("Off" | "All" | "One")[] = ["Off", "All", "One"];
        const currentIndex = modes.indexOf(this.repeatMode as "Off" | "All" | "One");
        const nextMode = modes[(currentIndex + 1) % modes.length];
        await this.setRepeatMode(nextMode);
    }

    async next() {
        await this.skipForward(1);
    }

    async previous() {
        await this.skipBackward(1);
    }
}

export const audioStore = new AudioStore();
