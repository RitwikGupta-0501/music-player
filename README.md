# Echo

A minimal, algorithmically-driven desktop music player for local audio files. Echo plays FLAC, MP3, WAV, OGG, and M4A from your filesystem, with an embedded Lua sandbox that lets community-written provider scripts resolve the next track via external APIs — turning any seed track into an infinite, intelligent radio.

> **Status:** `v0.1.0` — Core playback, library management, queue system, and extension sandbox are functional. UI polish and advanced features are in active development.

---

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Tech Stack](#tech-stack)
- [System Design](#system-design)
  - [Audio Engine](#audio-engine)
  - [Database Layer](#database-layer)
  - [Queue State Machine](#queue-state-machine)
  - [Extension Sandbox (Lua)](#extension-sandbox-lua)
  - [IPC Bridge](#ipc-bridge)
  - [Frontend](#frontend)
- [Database Schema](#database-schema)
- [IPC Command Reference](#ipc-command-reference)
- [Keyboard Shortcuts](#keyboard-shortcuts)
- [Provider SDK](#provider-sdk)
- [Concurrency Model](#concurrency-model)
- [Feature Flags](#feature-flags)
- [Telemetry & Diagnostics](#telemetry--diagnostics)
- [Project Structure](#project-structure)
- [Getting Started](#getting-started)
- [Build & Lint Commands](#build--lint-commands)
- [Roadmap](#roadmap)
- [License](#license)

---

## Architecture Overview

Echo follows a strict **daemon + remote control** architecture. The Rust backend owns all state: audio playback, the queue, the database, and the Lua extension sandbox. The SvelteKit frontend is a thin, stateless remote control that sends commands over Tauri IPC and reacts to event emissions.

```
┌─────────────────────────────────────────────────────────┐
│                     SvelteKit Frontend                  │
│  ┌──────────┐  ┌───────────┐  ┌───────────────────────┐ │
│  │AudioStore│  │LibraryStore│  │ UI Components (Svelte)│ │
│  └────┬─────┘  └─────┬─────┘  └───────────┬───────────┘ │
│       │              │                     │             │
│       └──────────────┼─────────────────────┘             │
│                      │ Tauri IPC (invoke / listen)       │
├──────────────────────┼───────────────────────────────────┤
│                      │        Rust Backend               │
│  ┌───────────────────┼─────────────────────────────────┐ │
│  │              AppState (Managed)                     │ │
│  │  ┌──────────┐  ┌──────────┐  ┌───────────────────┐ │ │
│  │  │Audio     │  │Database  │  │Queue State Machine│ │ │
│  │  │Thread    │  │Thread    │  │(in-memory + SQLite)│ │ │
│  │  │(rodio)   │  │(rusqlite)│  └───────────────────┘ │ │
│  │  └──────────┘  └──────────┘                        │ │
│  │  ┌──────────────────────┐                          │ │
│  │  │Lua Sandbox (mlua)    │                          │ │
│  │  │  + async reqwest     │                          │ │
│  │  └──────────────────────┘                          │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

**Key invariants:**
- The audio engine runs on a **dedicated OS thread** — never inside the Tokio runtime.
- All SQLite access is funneled through a **single database actor thread** via `mpsc` channels, preventing `SQLITE_BUSY` lock contention.
- The frontend never holds authoritative state. Every mutation goes through a Tauri command, and the backend emits `queue-changed` / `player-sync` events for the frontend to reconcile.

---

## Tech Stack

| Layer        | Technology                                                                 |
|--------------|----------------------------------------------------------------------------|
| **Frontend** | SvelteKit (SPA mode), Svelte 5 (Runes: `$state`, `$derived`, `$effect`), TypeScript |
| **Backend**  | Rust (2021 edition), Tauri 2.0                                             |
| **Runtime**  | Tokio (async I/O), dedicated OS threads for audio and database             |
| **Audio**    | `rodio` sink + `symphonia` codec engine (all-codecs, all-formats)          |
| **Database** | `rusqlite` (bundled SQLite) — embedded, zero-config                        |
| **Metadata** | `id3` for tag parsing, `walkdir` for recursive directory scanning          |
| **Extensions** | `mlua` (Lua 5.4, vendored) with sandboxed execution + `reqwest` HTTP bridge |
| **Icons**    | `lucide-svelte`                                                            |
| **Bundler**  | Vite 6, Bun                                                               |

---

## System Design

### Audio Engine

**Module:** `src-tauri/src/audio/`

The audio subsystem is completely isolated on its own OS thread, communicating exclusively through a lock-free `mpsc` channel. This guarantees that audio playback is never blocked by Tokio task scheduling or IPC latency.

**Command protocol (`AudioCommand` enum):**

| Command        | Payload       | Behavior                                                        |
|----------------|---------------|-----------------------------------------------------------------|
| `Load(path)`   | `String`      | Stops current playback, decodes file via Symphonia, appends to sink, auto-plays |
| `Play`         | —             | Resumes the sink                                                |
| `Pause`        | —             | Pauses the sink                                                 |
| `Stop`         | —             | Stops sink, clears track state                                  |
| `Seek(pos)`    | `f64` seconds | Re-opens file from seek position (Symphonia coarse seek), preserves pause state |
| `SetVolume(v)` | `f32` (0–1)   | Sets sink volume (respects mute state)                          |
| `SetMute(b)`   | `bool`        | Zeros volume on mute, restores on unmute                        |
| `Quit`         | —             | Breaks the event loop, thread exits                             |

**Decoder:** A custom `SymphoniaSource` wraps Symphonia's format reader and codec decoder, converting packets into interleaved `i16` PCM samples streamed through a `VecDeque` ring buffer. It implements `rodio::Source` for direct sink integration.

**Seek strategy:** Seeking re-opens the file and uses `SeekMode::Coarse` — Symphonia falls back to bisection search for formats without seek tables (e.g., FLAC without `SEEKTABLE`).

**Track-end detection:** The audio thread polls `sink.empty()` on each loop iteration. When the sink drains and a track was loaded, it emits a `track-ended` event to the frontend, which triggers auto-advance via the queue system.

**State synchronization:** The thread emits `player-sync` events (state, position, duration, track path) on every command that changes playback state.

---

### Database Layer

**Module:** `src-tauri/src/db/`

All database access is serialized through a **single actor thread** that owns the `rusqlite::Connection`. Tauri commands send `DbRequest` variants over an `mpsc` channel, each carrying a `tokio::sync::oneshot` sender for the response. This design:

- Eliminates `SQLITE_BUSY` errors during concurrent scanning + querying
- Keeps all `rusqlite` calls synchronous (no `spawn_blocking` needed inside the actor)
- Provides natural backpressure via channel capacity

**Supported operations (20 `DbRequest` variants):**

| Category         | Operations                                                            |
|------------------|-----------------------------------------------------------------------|
| **Library**      | `InsertTracks`, `GetLocalTracks`, `GetAlbums`, `GetAlbumTracks`, `ClearLocalLibrary`, `RemoveTrackByPath` |
| **Playlists**    | `CreatePlaylist`, `DeletePlaylist`, `RenamePlaylist`, `AddToPlaylist`, `RemoveFromPlaylist`, `GetPlaylists`, `GetPlaylistTracks`, `ReorderPlaylistTrack` |
| **Settings**     | `GetSetting`, `SetSetting`                                           |
| **Queue**        | `LoadAudioCache`                                                      |
| **Maintenance**  | `FactoryReset`                                                        |

**Library scanning** uses `walkdir` for recursive traversal and `id3::Tag` for metadata extraction. Scanning runs inside `tokio::task::spawn_blocking` to avoid blocking the async runtime, then sends the batch to the DB thread via `InsertTracks`.

---

### Queue State Machine

**Module:** `src-tauri/src/queue/`

The queue is the most complex subsystem, implemented as a deterministic state machine with full persistence and crash recovery.

**State shape (`QueueState`):**

```rust
struct QueueState {
    tracks: Vec<QueueTrack>,        // Ordered track list
    current_position: usize,        // Index into tracks
    repeat_mode: RepeatMode,        // Off | All | One
    mode: QueueMode,                // Normal | Shuffle
    shuffle_state: Option<ShuffleState>,  // Shuffle order + cursor
}
```

**Navigation matrix:**

| Mode     | Direction | At boundary (Repeat::Off) | At boundary (Repeat::All)       | Repeat::One            |
|----------|-----------|---------------------------|---------------------------------|------------------------|
| Normal   | Next      | `None` (stops)            | Wraps to index 0                | Stays at current index |
| Normal   | Prev      | `None` (stops)            | Wraps to last index             | Stays at current index |
| Shuffle  | Next      | `None` (stops)            | Regenerates shuffle, restarts   | Stays at current index |
| Shuffle  | Prev      | `None` (stops)            | Wraps to end of shuffle order   | Stays at current index |

**Shuffle implementation:**
- Generates a randomized permutation of `instance_id`s using `rand::thread_rng()`
- Tracks a `cursor` into the shuffle order, independent of `current_position` in the track array
- On `Repeat::All` boundary, regenerates the shuffle order for maximum variety
- Adding a track to a shuffled queue appends it to the shuffle order
- Toggling shuffle off preserves the currently playing track's position

**Reorder correctness:** When reordering tracks via drag-and-drop, the `current_position` is adjusted to follow the currently playing track:
- If the moved track *is* the current track, position follows it
- If the move crosses the current position, it shifts ±1 accordingly

**Persistence (Phase 5):** Queue state is persisted to SQLite tables (`queue_state`, `queued_tracks`, `shuffle_order`) and recovered on app startup. The system keeps the last 5 queue snapshots for recovery, cleaning up older states periodically.

**Test coverage:** 25+ unit tests cover: set/clear/add, forward/backward navigation in both modes, all repeat mode combinations, boundary conditions, reorder correctness, shuffle enable/disable, and single-track edge cases.

---

### Extension Sandbox (Lua)

**Module:** `src-tauri/src/providers/`

Echo embeds a Lua 5.4 runtime via `mlua` to allow community-authored provider scripts that can discover and resolve tracks from external APIs.

**Sandbox restrictions:**

| Constraint           | Implementation                                                        |
|----------------------|-----------------------------------------------------------------------|
| **Instruction limit** | `set_hook` triggers every 100,000 instructions → runtime error       |
| **Memory limit**     | `set_memory_limit(4MB)` — hard cap on Lua heap                       |
| **Blocked globals**  | `os`, `io`, `package`, `require`, `dofile`, `loadfile`, `load`, `debug` — all removed |
| **HTTP access**      | Single `http.get(url)` function injected, backed by `reqwest` with 10s timeout |

**Provider contract:** A Lua script returns a table with a `search(query)` function that returns an array of `{ id, title, artist, stream_url }` result tables. See the [Provider SDK](#provider-sdk) section for the full API.

**Lifecycle:** On first launch, bundled provider scripts are copied from the app resources to the user's data directory (`<app_data>/providers/`). The `ProviderManager` loads the active script and exposes its `search` function as an async Tauri command.

---

### IPC Bridge

Communication between frontend and backend uses Tauri 2.0's command invocation (`invoke`) for requests and event emission (`emit`/`listen`) for push updates.

**Event channels (backend → frontend):**

| Event            | Payload                                           | Frequency          |
|------------------|---------------------------------------------------|---------------------|
| `player-sync`   | `{ state, position, duration, track }`            | On every playback state change |
| `track-ended`   | `()`                                              | When sink drains    |
| `queue-changed` | `{ tracks, current_position, current_track, repeat_mode, queue_mode }` | On every queue mutation |

**Throttling strategy:** The frontend uses `requestAnimationFrame`-based interpolation for the progress bar rather than polling the backend, keeping IPC traffic to state change events only. The `AudioStore` maintains `_syncPosition` and `_syncTimestamp` and interpolates between sync events at 60fps.

---

### Frontend

**Module:** `src/`

The frontend is a SvelteKit SPA with Svelte 5 runes for reactive state management. It acts purely as a remote control — no authoritative state lives in the frontend.

**Store architecture:**

| Store               | Responsibilities                                                    |
|----------------------|---------------------------------------------------------------------|
| `AudioStore`         | Playback state, queue cache, volume, IPC event listeners, clock interpolation |
| `LibraryStore`       | Library metadata cache, album/track fetching, scan triggers         |
| `ToastStore`         | Notification queue for user-facing messages                         |
| `Keymap`             | Keyboard shortcut configuration with re-mappable bindings           |

**Component hierarchy:**

| Component            | Purpose                                                             |
|----------------------|---------------------------------------------------------------------|
| `PlayerBar`          | Transport controls, progress bar, volume, now-playing display       |
| `Sidebar`            | Navigation (Albums, Playlists, Settings)                            |
| `AlbumGrid`          | Paginated album card grid with lazy artwork loading                 |
| `AlbumCard`          | Individual album tile with cover art                                |
| `AlbumDetail`        | Track listing for a selected album                                  |
| `QueueSidebar`       | Current queue display with track list and reorder                   |
| `PlaylistView`       | Playlist listing                                                    |
| `PlaylistDetail`     | Tracks within a playlist                                            |
| `SettingsModal`      | App settings interface                                              |
| `KeyboardHandler`    | Global keyboard shortcut listener                                   |
| `ErrorBoundary`      | Graceful error recovery wrapper                                     |
| `ToastContainer`     | Toast notification display                                          |

**Artwork pipeline:** Album art is extracted from ID3 tags on-demand via the `extract_and_cache_artwork` command, cached as JPG files in `<app_data>/artwork/`, and served to the frontend via Tauri's asset protocol.

---

## Database Schema

```sql
-- Core library
albums    (id, title, artist, cover_art_path)           UNIQUE(title, artist)
tracks    (id, title, artist, album_id FK, track_number, file_path UNIQUE)
playlists (id, name UNIQUE, created_at)
playlist_tracks (id, playlist_id FK, track_id FK, position)
settings  (key PK, value)

-- Queue persistence
queue_state     (id, current_position, repeat_mode, queue_mode, created_at, updated_at)
queued_tracks   (id, queue_state_id FK, instance_id UNIQUE, track_id FK, position)
shuffle_order   (id, queue_state_id FK, instance_id, position, seed)
queue_history   (id, action, from_position, to_position, track_id, created_at)
queue_snapshots (id, queue_state_id FK, description, created_at)
```

Foreign keys are enforced (`PRAGMA foreign_keys = ON`). Cascading deletes are configured on playlist tracks and queue-related tables.

---

## IPC Command Reference

### Audio Commands

| Command           | Arguments              | Returns            |
|-------------------|------------------------|--------------------|
| `load_audio`      | `path: String`         | `Result<(), String>` |
| `play_audio`      | —                      | `Result<(), String>` |
| `pause_audio`     | —                      | `Result<(), String>` |
| `stop_audio`      | —                      | `Result<(), String>` |
| `seek_audio`      | `position: f64`        | `Result<(), String>` |
| `set_volume`      | `volume: f32`          | `Result<(), String>` |
| `set_mute`        | `mute: bool`           | `Result<(), String>` |

### Library Commands

| Command                    | Arguments                          | Returns                        |
|----------------------------|------------------------------------|--------------------------------|
| `scan_local_directory`     | `path: String`                     | `Result<usize, String>`        |
| `get_local_tracks`         | `limit: u32, offset: u32`         | `Result<Vec<LocalTrack>, String>` |
| `get_albums`               | `limit: u32, offset: u32`         | `Result<Vec<Album>, String>`   |
| `get_album_tracks`         | `album_id: i64, limit, offset`    | `Result<Vec<LocalTrack>, String>` |
| `extract_and_cache_artwork`| `track_id: i64, file_path: String` | `Result<Option<String>, String>` |
| `clear_local_library`      | —                                  | `Result<(), String>`           |
| `remove_track_by_path`     | `path: String`                     | `Result<(), String>`           |

### Playlist Commands

| Command                  | Arguments                            | Returns                        |
|--------------------------|--------------------------------------|--------------------------------|
| `get_playlists`          | `limit: u32, offset: u32`           | `Result<Vec<Playlist>, String>` |
| `create_playlist`        | `name: String`                       | `Result<i64, String>`          |
| `delete_playlist`        | `playlist_id: i64`                   | `Result<(), String>`           |
| `rename_playlist`        | `playlist_id: i64, new_name: String` | `Result<(), String>`           |
| `add_to_playlist`        | `playlist_id: i64, track_id: i64`    | `Result<(), String>`           |
| `remove_from_playlist`   | `playlist_id: i64, track_id: i64`    | `Result<(), String>`           |
| `get_playlist_tracks`    | `playlist_id: i64, limit, offset`    | `Result<Vec<LocalTrack>, String>` |
| `reorder_playlist_track` | `playlist_id, from_pos, to_pos`      | `Result<(), String>`           |

### Queue Commands

| Command             | Arguments                            | Returns                             |
|---------------------|--------------------------------------|-------------------------------------|
| `set_queue`         | `tracks: Vec<QueueTrack>, start_index: usize` | `Result<QueueChangeEvent, String>` |
| `add_to_queue`      | `track: QueueTrack`                  | `Result<QueueChangeEvent, String>` |
| `clear_queue`       | —                                    | `Result<QueueChangeEvent, String>` |
| `skip_forward`      | `count: u32`                         | `Result<QueueChangeEvent, String>` |
| `skip_backward`     | `count: u32`                         | `Result<QueueChangeEvent, String>` |
| `jump_to_position`  | `position: usize`                    | `Result<QueueChangeEvent, String>` |
| `jump_to_track`     | `instance_id: String`                | `Result<QueueChangeEvent, String>` |
| `reorder_queue`     | `from_index: usize, to_index: usize` | `Result<QueueChangeEvent, String>` |
| `set_repeat_mode`   | `mode: String` ("Off"/"All"/"One")   | `Result<QueueChangeEvent, String>` |
| `set_shuffle`       | `enabled: bool`                      | `Result<QueueChangeEvent, String>` |
| `get_queue`         | —                                    | `Result<QueueChangeEvent, String>` |
| `get_queue_length`  | —                                    | `Result<usize, String>`           |
| `get_current_track` | —                                    | `Result<Option<QueueTrack>, String>` |
| `reshuffle`         | —                                    | `Result<QueueChangeEvent, String>` |

### System Commands

| Command              | Arguments              | Returns                            |
|----------------------|------------------------|------------------------------------|
| `get_setting`        | `key: String`          | `Result<Option<String>, String>`   |
| `set_setting`        | `key, value: String`   | `Result<(), String>`               |
| `factory_reset`      | —                      | `Result<(), String>`               |
| `search_provider`    | `query: String`        | `Result<Vec<TrackResult>, String>` |
| `get_error_log`      | —                      | `Vec<ErrorEvent>`                  |
| `get_error_count`    | —                      | `u64`                              |
| `clear_error_log`    | —                      | `()`                               |
| `get_feature_flags`  | —                      | `Vec<String>`                      |
| `is_feature_enabled` | `flag: String`         | `bool`                             |
| `set_feature_enabled`| `flag: String, enabled: bool` | `()`                        |

---

## Keyboard Shortcuts

| Action          | Default Binding   |
|-----------------|-------------------|
| Play / Pause    | `Space`           |
| Seek Back 5s    | `←`               |
| Seek Forward 5s | `→`               |
| Previous Track  | `Ctrl + ←`        |
| Next Track      | `Ctrl + →`        |
| Volume Up       | `Ctrl + ↑`        |
| Volume Down     | `Ctrl + ↓`        |
| Toggle Shuffle  | `Ctrl + S`        |
| Cycle Repeat    | `Ctrl + R`        |
| Close / Back    | `Escape`          |

Keybindings are designed for re-mappability — the keymap store can be persisted to the SQLite `settings` table for user customization.

---

## Provider SDK

Providers are Lua 5.4 scripts that implement a standard interface. Place `.lua` files in `<app_data>/providers/`.

### Minimal Provider

```lua
local provider = {}

function provider.search(query)
    -- Return an array of track results
    return {
        { id = "1", title = "Track Name", artist = "Artist", stream_url = "" }
    }
end

return provider
```

### Full Provider Template

```lua
local provider = {}

provider.metadata = {
    name = "My Provider",
    author = "Your Name",
    version = "1.0.0",
    features = { "search", "stream", "radio", "lyrics" }
}

function provider.search(query, type)
    -- type: "track", "album", or "artist"
    -- Returns: { { id, title, artist, duration_ms, art_url }, ... }
end

function provider.get_stream(track_id)
    -- Returns: { url = "https://...", format = "mp3", bitrate = 320 }
end

function provider.get_recommendations(seed_track_id)
    -- Returns an array of similar tracks
end

function provider.get_lyrics(track_id)
    -- Returns: { synced = true, content = "[00:15.00]First line\n..." }
end

return provider
```

### Available Globals

| Global      | Description                                   |
|-------------|-----------------------------------------------|
| `http.get(url)` | Async HTTP GET, returns response body as string. 10s timeout. |
| `string.*`  | Standard Lua string library                   |
| `table.*`   | Standard Lua table library                    |
| `math.*`    | Standard Lua math library                     |
| `tonumber`  | Type conversion                               |
| `tostring`  | Type conversion                               |
| `type`      | Type introspection                            |
| `pairs`     | Iterator                                      |
| `ipairs`    | Array iterator                                |
| `pcall`     | Protected call                                |

**Blocked:** `os`, `io`, `package`, `require`, `dofile`, `loadfile`, `load`, `debug`

**Limits:** 100,000 instruction count cap, 4MB memory ceiling.

---

## Concurrency Model

Echo uses a hybrid concurrency model with three execution contexts:

```
┌──────────────────────────────────────────────────┐
│                Tokio Async Runtime               │
│  ┌──────────────────────────────────────────────┐│
│  │  Tauri IPC handlers (async commands)         ││
│  │  Lua provider search (async reqwest)         ││
│  │  spawn_blocking: directory scanning, ID3     ││
│  └──────────────────────────────────────────────┘│
├──────────────────────────────────────────────────┤
│            Dedicated OS Thread: Audio            │
│  rodio Sink + Symphonia decoder                  │
│  Communication: mpsc::Receiver<AudioCommand>     │
│  Emits: player-sync, track-ended events          │
├──────────────────────────────────────────────────┤
│           Dedicated OS Thread: Database          │
│  rusqlite Connection (single owner)              │
│  Communication: mpsc::Receiver<DbRequest>        │
│  Responses: oneshot::Sender per request          │
└──────────────────────────────────────────────────┘
```

**Design rationale:**
- `rodio` is `!Send` — it cannot be moved between threads or held across `.await` points.
- `rusqlite` is synchronous — running it inside `async` functions would block the Tokio runtime.
- Dedicated threads provide deterministic, low-latency processing free from task scheduler interference.

---

## Feature Flags

Runtime feature flags control access to experimental or gated functionality:

| Flag               | Default  | Description                                |
|--------------------|----------|--------------------------------------------|
| `ShuffleMode`      | Enabled  | Shuffle queue navigation mode              |
| `ReorderQueue`     | Enabled  | Drag-and-drop queue reordering             |
| `VirtualScrolling` | Enabled  | Virtual list rendering for large libraries |
| `PersistentQueue`  | Enabled  | Queue state persistence across restarts    |

Flags can be queried and toggled at runtime via the `is_feature_enabled` / `set_feature_enabled` IPC commands.

---

## Telemetry & Diagnostics

Echo includes a lightweight, **local-only** telemetry system for debugging. No data leaves the device.

- **Error log:** Ring buffer of the last 1,000 errors, each with category, message, and timestamp.
- **Error counter:** Atomic counter for total errors since launch.
- **IPC access:** `get_error_log`, `get_error_count`, `clear_error_log` commands.

Errors from queue operations, shuffle state transitions, and other critical paths are automatically recorded.

---

## Project Structure

```
echo-desktop/
├── src/                          # SvelteKit frontend
│   ├── app.css                   # Global styles
│   ├── app.html                  # HTML shell
│   ├── lib/
│   │   ├── components/           # Svelte 5 components (12 total)
│   │   │   ├── PlayerBar.svelte
│   │   │   ├── Sidebar.svelte
│   │   │   ├── AlbumGrid.svelte
│   │   │   ├── AlbumCard.svelte
│   │   │   ├── AlbumDetail.svelte
│   │   │   ├── QueueSidebar.svelte
│   │   │   ├── PlaylistView.svelte
│   │   │   ├── PlaylistDetail.svelte
│   │   │   ├── SettingsModal.svelte
│   │   │   ├── KeyboardHandler.svelte
│   │   │   ├── ErrorBoundary.svelte
│   │   │   └── ToastContainer.svelte
│   │   └── stores/               # Svelte 5 rune-based stores
│   │       ├── audio.svelte.ts   # Playback + queue state
│   │       ├── library.svelte.ts # Library metadata cache
│   │       ├── toast.svelte.ts   # Notification system
│   │       └── keymap.ts         # Keyboard shortcut config
│   └── routes/                   # SPA route (single page)
│       ├── +layout.svelte
│       ├── +layout.ts
│       └── +page.svelte
├── src-tauri/                    # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs               # Entry point
│   │   ├── lib.rs                 # App setup, state, IPC handlers (~480 LOC)
│   │   ├── feature_flags.rs       # Runtime feature flag system
│   │   ├── telemetry.rs           # Error logging and diagnostics
│   │   ├── audio/
│   │   │   ├── mod.rs             # Audio thread + rodio sink loop
│   │   │   ├── commands.rs        # Tauri audio IPC commands
│   │   │   └── symphonia_source.rs # Custom Symphonia → rodio adapter
│   │   ├── db/
│   │   │   ├── mod.rs             # Database actor thread
│   │   │   ├── schema.rs          # Table creation + migrations
│   │   │   └── queries.rs         # SQL query implementations
│   │   ├── providers/
│   │   │   ├── mod.rs             # Lua runtime + provider manager
│   │   │   └── sandbox.rs         # Sandbox configuration
│   │   └── queue/
│   │       ├── mod.rs             # Queue state machine (~1000 LOC, 25+ tests)
│   │       ├── commands.rs        # Queue IPC commands + event emission
│   │       ├── persistence.rs     # SQLite save/load for queue state
│   │       └── recovery.rs        # Startup recovery logic
│   └── benches/
│       └── queue_performance.rs   # Criterion benchmarks
├── providers/                    # Bundled Lua provider scripts
│   ├── dummy_search.lua
│   └── template.lua
├── docs/                         # Design documentation
│   ├── 1-features.md
│   ├── 2-screens.md
│   ├── 3-design-system.md
│   ├── edge-cases-playback.md
│   └── feature-roadmap.md
└── static/                       # Static assets
```

**Codebase size:** ~3,300 lines Rust, ~3,350 lines TypeScript/Svelte.

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Bun](https://bun.sh/) (JavaScript runtime & package manager)
- System audio libraries (platform-dependent, required by `rodio`):
  - **Linux:** `libasound2-dev` (ALSA)
  - **macOS:** CoreAudio (included with Xcode)
  - **Windows:** No additional dependencies

### Setup

```bash
# Clone the repository
git clone https://github.com/RitwikGupta/echo-desktop.git
cd echo-desktop

# Install frontend dependencies
bun install

# Run in development mode (starts both Vite dev server and Tauri backend)
bun run tauri dev
```

### Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) with extensions:
- [Svelte for VS Code](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

---

## Build & Lint Commands

| Command                                               | Purpose                                |
|-------------------------------------------------------|----------------------------------------|
| `bun run dev`                                         | Start frontend dev server (Vite)       |
| `bun run tauri dev`                                   | Full development mode (frontend + Rust)|
| `bun run check`                                       | Svelte type-checking + integrity       |
| `cargo clippy --all-targets -- -D warnings`           | Rust linting (zero warnings policy)    |
| `cargo test`                                          | Run Rust unit tests                    |
| `cargo bench`                                         | Run Criterion performance benchmarks   |
| `bun run build`                                       | Production frontend build              |
| `bun run tauri build`                                 | Full production build with bundling    |

---

## Roadmap

### Completed ✅
- Audio engine with rodio + Symphonia (FLAC/MP3/WAV/OGG/M4A)
- Local library scanning with ID3 metadata extraction
- Relational database (albums, tracks, playlists)
- Queue state machine with shuffle, repeat, reorder
- Queue persistence and crash recovery
- Interpolation-based progress bar (60fps, zero IPC polling)
- Lua extension sandbox with HTTP bridge
- Keyboard shortcuts with re-mappable bindings
- Feature flags and telemetry systems
- Criterion benchmarks for queue performance

### In Progress 🔧
- UI overhaul with consistent design language
- Comprehensive settings page
- Virtual scrolling for large libraries

### Planned 📋
- Global search (Ctrl+K) with fuzzy matching
- Live synced lyrics via Lua providers
- Smart shuffle (genre/BPM-aware)
- Dynamic theming from album artwork
- MPRIS / SMTC / NowPlaying OS integration
- Equalizer and crossfade
- Provider marketplace
- **Expanded Plugin Ecosystem**: Scaling the Lua sandbox into themes (CSS variable injection over IPC) and UI Features (declarative JSON schemas for custom context menu actions) without breaking the strict "Dumb Frontend" constraints.

---

## License

MIT
