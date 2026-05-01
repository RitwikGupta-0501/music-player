# Echo Music Player - Project Context & Architecture Documentation

## 1. Project Overview
**Echo** is a high-performance, algorithmically-driven, extensible desktop music player. Designed with a deep focus on systems-level performance, the core engine leverages Rust for memory-safe concurrency, while relying on a lightweight SvelteKit frontend for a premium desktop UI. 

The application is built to handle massive local music libraries (FLAC/MP3/WAV) seamlessly, while simultaneously supporting an extensible plugin architecture that runs sandboxed third-party scripts to resolve network streams and algorithmic queues. 

## 2. Tech Stack & Architecture
* **Frontend:** SvelteKit (SPA mode), Svelte 5 (Runes exclusively: `$state`, `$derived`, `$effect`), TypeScript, Vite, Bun.
* **Backend:** Rust, Tauri 2.0.
* **Async Runtime:** Tokio.
* **Audio Engine:** `rodio` (running on a dedicated, non-blocking background thread communicating via `mpsc::channel`).
* **Database:** `rusqlite` (bundled, local caching for library, playlists, and metadata).
* **Provider Engine (Extensions):** `mlua` (embedded Lua 5.4 interpreter), executing sandboxed Lua scripts with a Rust-injected async HTTP client (`reqwest`).

### Core Architectural Patterns
1. **The Audio Daemon:** The audio sink lives strictly inside an isolated background thread. It communicates with the UI purely through channel messages (`Load`, `Play`, `Pause`, `Stop`) to prevent UI lag or database reads from causing audio stuttering.
2. **The IPC Bridge:** Tauri's event system (`app_handle.emit`) pushes real-time state (`player-state`, `current-track`) to the frontend.
3. **The Local Scanner:** Fast native directory traversal using `walkdir` and metadata extraction using `id3`, batched into SQLite.
4. **The Lua Sandbox:** A secure environment for parsing dynamic network sources. Rust evaluates the script, invokes functions asynchronously, and deserializes the returned Lua tables directly into strongly-typed Rust structs.

## 3. Current Implementation Status
* **Phase 1 (Complete):** Audio engine is fully wired. Background thread manages state, channel receives commands, audio plays locally.
* **Phase 2 (Complete):** Tokio async runtime and SQLite database initialized. Two-way IPC events broadcast playback state to the UI.
* **Phase 3 (Complete):** Lua provider bridge built. Rust successfully boots `mlua`, injects `reqwest` for HTTP fetching, and evaluates dummy scripts that hit external APIs and return track payloads.
* **Phase 4 (In Progress):** Local library backend. SQLite schema expanded for directories, tracks, and playlists. Native `walkdir` + `id3` scanner implemented as Tauri commands. 

## 4. Immediate Goals (What we want to achieve next)
1. **Frontend Local Library Wiring:** Connect the Svelte UI to the newly created `scan_local_directory`, `get_local_tracks`, and `create_playlist` Rust commands.
2. **Algorithmic Queue Engine:** Expand the database queries or Lua provider functions to support an "Infinite Queue" (e.g., automatically finding related tracks when the playlist runs out).
3. **UI/UX Implementation:** Transition from basic HTML buttons to the final "Cinematic Chalk on Slate" aesthetic. The design should feature deep dark mineral tones, subtle textures, and a clean, unobtrusive glassmorphic player overlay.

## 5. Development Guidelines & Constraints for AI Agents
When generating code or architecture suggestions for this project, adhere to the following principles:

* **Backend-First Philosophy:** Prioritize robust backend engineering, low-level system optimization, and intelligent architecture over heavy frontend manipulation. Let Rust handle the heavy lifting (caching, sorting, scanning, network polling).
* **Linux/Power-User Targeting:** Assume the deployment environment is an advanced Linux system (e.g., tiling window managers, minimal overhead). Ensure dependencies and UI scaling respect desktop environments.
* **No Deprecated Svelte:** Absolutely no Svelte 4 syntax. Use Svelte 5 runes for all reactivity.
* **Non-Blocking Execution:** Never block the audio thread. All file I/O, network requests, and database queries must run on Tokio's async tasks or separate threads.
* **Data Privacy / Local-First:** Keep as much data local as possible. Rely on the embedded SQLite database for state persistence. Do not force integrations with commercial streaming services unless explicitly utilizing the sandboxed Lua provider engine.
