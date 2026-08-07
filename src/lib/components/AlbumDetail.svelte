<script lang="ts">
    import { libraryStore, type Album, type LocalTrack } from "$lib/stores/library.svelte";
    import { audioStore } from "$lib/stores/audio.svelte";
    import { toastStore } from "$lib/stores/toast.svelte";
    import { DotsThree, Play, Plus, Waveform } from "phosphor-svelte";

    let { album, onBack } = $props<{ album: Album; onBack: () => void }>();

    let tracks = $state<LocalTrack[]>([]);
    let artUrl = $state<string | null>(null);

    let activeDropdown = $state<number | null>(null);
    let isCreatingPlaylistForTrack = $state<number | null>(null);
    let newPlaylistName = $state("");

    $effect(() => {
        libraryStore.getAlbumTracks(album.id).then(t => {
            tracks = t;
            if (t.length > 0) {
                libraryStore.getArtworkUrl(t[0].id, t[0].file_path).then(url => (artUrl = url));
            }
        });

        const closeDropdowns = () => {
            activeDropdown = null;
            isCreatingPlaylistForTrack = null;
        };
        document.addEventListener("click", closeDropdowns);
        return () => document.removeEventListener("click", closeDropdowns);
    });

    function toggleDropdown(e: Event, index: number) {
        e.stopPropagation();
        activeDropdown = activeDropdown === index ? null : index;
        if (activeDropdown !== index) isCreatingPlaylistForTrack = null;
    }

    async function addTrackToPlaylist(e: Event, playlistId: number, trackId: number) {
        e.stopPropagation();
        await libraryStore.addToPlaylist(playlistId, trackId);
        activeDropdown = null;
    }

    async function createAndAddPlaylist(e: Event, trackId: number) {
        e.stopPropagation();
        if (!newPlaylistName.trim()) return;
        await libraryStore.createPlaylist(newPlaylistName);
        const newPlaylist = libraryStore.playlists.find(p => p.name === newPlaylistName);
        if (newPlaylist) {
            await libraryStore.addToPlaylist(newPlaylist.id, trackId);
        }
        newPlaylistName = "";
        isCreatingPlaylistForTrack = null;
        activeDropdown = null;
    }

    async function playTrack(index: number) {
        if (!album || tracks.length === 0) return;

        if (audioStore.queue.length > 0) {
            const trackPayload = {
                id: tracks[index].id,
                title: tracks[index].title,
                artist: tracks[index].artist,
                album: album.title,
                file_path: tracks[index].file_path,
                track_number: tracks[index].track_number,
            };

            if (audioStore.trackClickBehavior === "interrupt") {
                await audioStore.playInterrupt(trackPayload);
                return;
            } else if (audioStore.trackClickBehavior === "append") {
                await audioStore.addToQueue(trackPayload);
                toastStore.show("Added to queue", 'info', 1500);
                return;
            }
        }

        const queueTracks = tracks.map((t) => ({
            id: t.id,
            title: t.title,
            artist: t.artist,
            album: album.title,
            file_path: t.file_path,
            track_number: t.track_number,
        }));
        
        await audioStore.setQueue(queueTracks, index);
    }
</script>

<div class="view-album">
    <div class="album-header">
        <div class="art-container" style={artUrl ? `background-image: url('${artUrl}');` : ''}>
            {#if !artUrl}
                <div class="art-placeholder"></div>
            {/if}
        </div>
        <div class="album-info">
            <h3 class="album-title font-headline-lg">{album.title}</h3>
            <p class="album-artist">{album.artist || "Unknown Artist"}</p>
        </div>
    </div>

    <div class="track-list">
        {#each tracks as track, i}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div 
                class="track-row" 
                class:active={audioStore.currentTrack === track.title || audioStore.currentTrack === track.file_path}
                onclick={() => playTrack(i)}
            >
                <div class="track-left">
                    <div class="track-status">
                        {#if audioStore.currentTrack === track.title || audioStore.currentTrack === track.file_path}
                            <div class="playing-visualizer">
                                <div class="bar"></div>
                                <div class="bar"></div>
                                <div class="bar"></div>
                                <div class="bar"></div>
                            </div>
                        {:else}
                            <span class="track-number">{track.track_number || i + 1}</span>
                            <Play size={18} weight="fill" class="track-play-icon" />
                        {/if}
                    </div>
                    <div class="track-name">{track.title}</div>
                </div>

                <div class="track-right">
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <button
                        class="more-btn"
                        onclick={(e) => toggleDropdown(e, i)}
                        title="Add to playlist"
                    >
                        <DotsThree size={20} weight="bold" />
                    </button>

                    {#if activeDropdown === i}
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <div
                            class="dropdown glass"
                            onclick={(e) => e.stopPropagation()}
                        >
                            <button
                                class="dropdown-row"
                                onclick={(e) => {
                                    e.stopPropagation();
                                    audioStore.playNext(track);
                                    activeDropdown = null;
                                }}
                            >
                                Play next
                            </button>
                            <button
                                class="dropdown-row"
                                onclick={(e) => {
                                    e.stopPropagation();
                                    audioStore.addToQueue(track);
                                    activeDropdown = null;
                                }}
                            >
                                Add to queue
                            </button>
                            <div class="dropdown-divider"></div>

                            {#if libraryStore.playlists.length > 0}
                                <div class="dropdown-section-label">Add to playlist</div>
                                {#each libraryStore.playlists as playlist}
                                    <button
                                        class="dropdown-row"
                                        onclick={(e) => addTrackToPlaylist(e, playlist.id, track.id)}
                                    >
                                        {playlist.name}
                                    </button>
                                {/each}
                                <div class="dropdown-divider"></div>
                            {/if}

                            {#if isCreatingPlaylistForTrack === i}
                                <div class="new-playlist-form" onclick={(e) => e.stopPropagation()}>
                                    <input
                                        type="text"
                                        bind:value={newPlaylistName}
                                        placeholder="Playlist name"
                                    />
                                    <button
                                        class="primary"
                                        style="padding: 0.35rem 0.65rem; font-size: 0.75rem; border-radius: 7px;"
                                        onclick={(e) => createAndAddPlaylist(e, track.id)}
                                    >
                                        Create
                                    </button>
                                </div>
                            {:else}
                                <button
                                    class="dropdown-row new-row"
                                    onclick={(e) => { e.stopPropagation(); isCreatingPlaylistForTrack = i; }}
                                >
                                    <Plus size={12} weight="bold" />
                                    New Playlist
                                </button>
                            {/if}
                        </div>
                    {/if}
                </div>
            </div>
        {/each}
    </div>
</div>

<style>
    .view-album {
        display: flex;
        flex-direction: column;
        position: absolute;
        inset: 0;
        padding: 1.5rem; /* p-6 */
        padding-bottom: 8rem; /* pb-32 */
    }

    .album-header {
        display: flex;
        align-items: center;
        gap: 1.5rem; /* gap-6 */
        margin-bottom: 2rem; /* mb-8 */
    }

    .art-container {
        width: 7rem; /* w-28 */
        height: 7rem; /* h-28 */
        flex-shrink: 0;
        border-radius: 1rem;
        background-color: #27272a; /* bg-zinc-800 fallback */
        background-size: cover;
        background-position: center;
        box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.5); /* shadow-lg */
        border: 1px solid rgba(255, 255, 255, 0.1);
    }

    .art-placeholder {
        width: 100%;
        height: 100%;
        background-color: var(--echo-raised);
        border-radius: 1rem;
    }

    .album-info {
        display: flex;
        flex-direction: column;
    }

    .album-title {
        font-size: 1.5rem; /* text-2xl */
        color: var(--echo-text-1);
        margin-bottom: 0.25rem; /* mb-1 */
        line-height: 1.2;
    }

    .album-artist {
        font-size: 0.875rem; /* text-sm */
        color: var(--echo-text-2);
        margin: 0;
        font-family: var(--echo-font-body);
    }

    .track-list {
        flex: 1;
        display: flex;
        flex-direction: column;
    }

    .track-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.75rem; /* p-3 */
        border-radius: 0.75rem; /* rounded-xl */
        cursor: pointer;
        transition: all 0.2s ease;
        position: relative;
    }

    .track-row:hover {
        background-color: rgba(255, 255, 255, 0.03);
    }

    .track-left {
        display: flex;
        align-items: center;
    }

    .track-status {
        width: 2rem; /* w-8 */
        font-size: 0.875rem; /* text-sm */
        color: var(--echo-text-2); /* text-muted */
        display: flex;
        align-items: center;
    }

    .track-number {
        display: block;
    }

    :global(.track-play-icon) {
        display: none;
        color: var(--echo-primary-dark);
    }

    .track-row:hover .track-number {
        display: none;
    }

    .track-row:not(.active):hover :global(.track-play-icon) {
        display: block;
    }

    .track-name {
        font-size: 0.9375rem; /* text-[15px] */
        font-weight: 500;
        color: var(--echo-text-1);
        transition: color 0.2s ease;
        font-family: var(--echo-font-body);
    }

    .track-row:hover .track-name {
        color: #ffffff;
    }

    .track-row.active    .track-number {
        width: 1.5rem;
        text-align: right;
        color: var(--echo-text-3);
        font-variant-numeric: tabular-nums;
        font-size: 0.875rem;
    }

    .playing-visualizer {
        display: flex;
        align-items: flex-end;
        justify-content: center;
        gap: 2px;
        height: 14px;
        width: 1.5rem;
    }

    .playing-visualizer .bar {
        width: 3px;
        background-color: var(--echo-primary-dark);
        border-radius: 2px;
        transform-origin: bottom;
    }

    .playing-visualizer .bar:nth-child(1) { height: 100%; animation: eq-bar-1 1.2s ease-in-out infinite; }
    .playing-visualizer .bar:nth-child(2) { height: 100%; animation: eq-bar-2 1.5s ease-in-out infinite; }
    .playing-visualizer .bar:nth-child(3) { height: 100%; animation: eq-bar-3 1.1s ease-in-out infinite; }
    .playing-visualizer .bar:nth-child(4) { height: 100%; animation: eq-bar-4 1.4s ease-in-out infinite; }

    @keyframes eq-bar-1 {
        0%, 100% { transform: scaleY(0.3); }
        25% { transform: scaleY(0.9); }
        50% { transform: scaleY(0.5); }
        75% { transform: scaleY(1.0); }
    }

    @keyframes eq-bar-2 {
        0%, 100% { transform: scaleY(0.6); }
        25% { transform: scaleY(0.2); }
        50% { transform: scaleY(1.0); }
        75% { transform: scaleY(0.4); }
    }

    @keyframes eq-bar-3 {
        0%, 100% { transform: scaleY(0.8); }
        25% { transform: scaleY(0.4); }
        50% { transform: scaleY(0.9); }
        75% { transform: scaleY(0.3); }
    }

    @keyframes eq-bar-4 {
        0%, 100% { transform: scaleY(0.4); }
        25% { transform: scaleY(1.0); }
        50% { transform: scaleY(0.3); }
        75% { transform: scaleY(0.8); }
    }

    .track-row.active .track-status,
    .track-row.active .track-name {
        color: var(--echo-primary-dark); /* text-[#B58E62] */
    }

    .track-right {
        display: flex;
        align-items: center;
        opacity: 0;
        transition: opacity 0.2s ease;
    }

    .track-row:hover .track-right {
        opacity: 1;
    }

    .more-btn {
        background: transparent;
        border: none;
        color: var(--echo-text-2);
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 0.25rem;
        border-radius: 0.25rem;
    }

    .more-btn:hover {
        color: var(--echo-text-1);
        background-color: rgba(255, 255, 255, 0.05);
    }

    /* ── Dropdown menu ── */
    .dropdown {
        position: absolute;
        right: 0.5rem;
        top: 2.25rem;
        width: 210px;
        border-radius: 10px;
        z-index: 20;
        overflow: hidden;
        display: flex;
        flex-direction: column;
        background: rgba(22, 22, 28, 0.95) !important;
    }

    .dropdown-section-label {
        padding: 0.5rem 0.875rem 0.25rem;
        font-size: 0.68rem;
        font-weight: 600;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: var(--echo-text-3);
    }

    .dropdown-row {
        width: 100%;
        text-align: left;
        background: transparent;
        border: none;
        border-radius: 0;
        color: var(--echo-text-1);
        font-size: 0.8rem;
        font-weight: 400;
        padding: 0.5rem 0.875rem;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 0.4rem;
        transition: background 0.1s ease;
    }

    .dropdown-row:hover {
        background: rgba(255, 255, 255, 0.06);
    }

    .new-row { color: var(--echo-text-2); }
    .new-row:hover { color: var(--echo-text-1); }

    .dropdown-divider {
        height: 1px;
        background: var(--echo-border);
        margin: 0.25rem 0;
    }

    .new-playlist-form {
        display: flex;
        gap: 0.4rem;
        padding: 0.5rem 0.875rem;
        align-items: center;
    }

    .new-playlist-form input {
        font-size: 0.775rem;
        padding: 0.3rem 0.5rem;
        border-radius: 6px;
        flex: 1;
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.1);
        color: white;
    }
</style>

