<script lang="ts">
    import { libraryStore, type Album, type LocalTrack } from "$lib/stores/library.svelte";
    import { audioStore } from "$lib/stores/audio.svelte";
    import { ArrowLeft, MoreHorizontal, Play, Plus } from "lucide-svelte";

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

    function playTrack(index: number) {
        audioStore.setQueue(
            tracks.map(t => ({
                id: t.id,
                title: t.title,
                artist: t.artist,
                file_path: t.file_path,
            })),
            index
        );
    }
</script>

<!-- Back navigation -->
<button class="back-btn ghost" onclick={onBack}>
    <ArrowLeft size={15} strokeWidth={1.5} />
    <span>Library</span>
</button>

<!-- Editorial album header -->
<header class="album-header">
    <div class="header-art">
        {#if artUrl}
            <img src={artUrl} alt={album.title} />
        {:else}
            <svg viewBox="0 0 120 120" class="art-placeholder" aria-hidden="true">
                <circle cx="60" cy="60" r="58" fill="rgba(255 255 255 / 0.03)" stroke="rgba(255 255 255 / 0.06)" stroke-width="1" />
                <circle cx="60" cy="60" r="40" fill="rgba(255 255 255 / 0.02)" stroke="rgba(255 255 255 / 0.05)" stroke-width="0.75" />
                <circle cx="60" cy="60" r="22" fill="rgba(255 255 255 / 0.02)" stroke="rgba(255 255 255 / 0.04)" stroke-width="0.75" />
                <circle cx="60" cy="60" r="7" fill="rgba(255 255 255 / 0.08)" />
                <circle cx="60" cy="60" r="3" fill="rgba(255 255 255 / 0.12)" />
            </svg>
        {/if}
    </div>

    <div class="header-meta">
        <p class="meta-label">Album</p>
        <h1 class="album-title">{album.title}</h1>
        <p class="album-artist">{album.artist || "Unknown Artist"}</p>
        <p class="album-track-count">{tracks.length} {tracks.length === 1 ? "track" : "tracks"}</p>

        {#if tracks.length > 0}
            <button class="play-all primary" onclick={() => playTrack(0)}>
                <Play size={14} fill="currentColor" />
                Play All
            </button>
        {/if}
    </div>
</header>

<!-- Track list -->
{#if tracks.length > 0}
    <section class="track-section">
        <div class="track-list">
            {#each tracks as track, i}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                    class="track-card"
                    class:now-playing={audioStore.currentTrack === track.file_path}
                    onclick={() => playTrack(i)}
                    style="position: relative;"
                >
                    <div class="track-left">
                        <span class="track-num">{track.track_number || i + 1}</span>
                        <span class="track-name">{track.title}</span>
                    </div>

                    <div class="track-right">
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <button
                            class="more-btn"
                            onclick={(e) => toggleDropdown(e, i)}
                            title="Add to playlist"
                        >
                            <MoreHorizontal size={14} />
                        </button>

                        {#if activeDropdown === i}
                            <!-- svelte-ignore a11y_click_events_have_key_events -->
                            <div
                                class="dropdown glass"
                                onclick={(e) => e.stopPropagation()}
                            >
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
                                        <Plus size={12} />
                                        New Playlist
                                    </button>
                                {/if}
                            </div>
                        {/if}
                    </div>
                </div>
            {/each}
        </div>
    </section>
{/if}

<style>
    .back-btn {
        display: inline-flex;
        align-items: center;
        gap: 0.4rem;
        font-size: 0.8rem;
        color: var(--echo-text-2);
        margin-bottom: 2.5rem;
        padding: 0.4rem 0;
        background: transparent;
        border: none;
        transition: color 0.12s ease;
    }
    .back-btn:hover {
        color: var(--echo-text-1);
        background: transparent;
    }
    .back-btn:active { transform: none; }

    /* ── Album header ── */
    .album-header {
        display: flex;
        gap: 2.25rem;
        align-items: flex-end;
        margin-bottom: 3rem;
    }

    .header-art {
        width: 210px;
        height: 210px;
        flex-shrink: 0;
        border-radius: 12px;
        overflow: hidden;
        background: var(--echo-raised);
        border: 1px solid var(--echo-border);
        display: flex;
        align-items: center;
        justify-content: center;
        box-shadow: 0 16px 48px rgba(0 0 0 / 0.5);
    }

    .header-art img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
    }

    .art-placeholder {
        width: 70%;
        height: 70%;
        opacity: 0.6;
    }

    .header-meta {
        flex: 1;
        min-width: 0;
    }

    .meta-label {
        font-size: 0.7rem;
        font-weight: 600;
        letter-spacing: 0.1em;
        text-transform: uppercase;
        color: var(--echo-text-3);
        margin-bottom: 0.6rem;
    }

    .album-title {
        font-size: 2.4rem;
        font-weight: 700;
        letter-spacing: -0.04em;
        color: var(--echo-text-1);
        line-height: 1.05;
        margin-bottom: 0.5rem;
        /* Ensure no descender clip for display size */
        padding-bottom: 0.1em;
    }

    .album-artist {
        font-size: 1rem;
        color: var(--echo-text-2);
        font-weight: 400;
        margin-bottom: 0.35rem;
    }

    .album-track-count {
        font-size: 0.775rem;
        color: var(--echo-text-3);
        margin-bottom: 1.5rem;
    }

    .play-all {
        font-size: 0.8rem;
        padding: 0.55rem 1.1rem;
        border-radius: 8px;
    }

    /* ── Track rows ── */
    .track-section {
        margin-top: 0;
    }

    .track-left {
        display: flex;
        align-items: center;
        gap: 0.875rem;
        min-width: 0;
        flex: 1;
    }

    .track-num {
        font-size: 0.72rem;
        color: var(--echo-text-3);
        min-width: 20px;
        text-align: right;
        flex-shrink: 0;
        font-variant-numeric: tabular-nums;
    }

    .track-name {
        font-size: 0.85rem;
        font-weight: 450;
        color: var(--echo-text-1);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .track-card.now-playing .track-name {
        color: var(--echo-silver);
    }

    .track-right {
        display: flex;
        align-items: center;
        gap: 0.25rem;
        flex-shrink: 0;
    }

    .more-btn {
        background: transparent;
        border: none;
        color: var(--echo-text-3);
        padding: 0.25rem;
        border-radius: 5px;
        cursor: pointer;
        opacity: 0;
        display: flex;
        align-items: center;
        transition: opacity 0.12s ease, color 0.12s ease;
    }

    .track-card:hover .more-btn {
        opacity: 1;
    }

    .more-btn:hover {
        color: var(--echo-text-1);
        background: rgba(255 255 255 / 0.06);
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
        /* Override .glass for slightly more opaque dropdown */
        background: rgba(22 22 28 / 0.95) !important;
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
        background: rgba(255 255 255 / 0.06);
        transform: none;
    }

    .dropdown-row:active { transform: none; }

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
    }
</style>
