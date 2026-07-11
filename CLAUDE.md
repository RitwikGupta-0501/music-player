# Echo Music Player - Project Context & Governance

## System Intent & AI Directives
**Identity:** Echo is a minimal, algorithmically-driven local audio player (FLAC/MP3/WAV). 
**Core Loop:** Play a seed track -> Sandboxed Lua extension (`mlua`) dynamically resolves the next track via async `reqwest` APIs. 
**Anti-Goals:** Do NOT build manual playlist managers, massive frontend data stores, or heavy UI layouts. The UI is exclusively a remote control for the Rust daemon.

## Tech Stack & Architecture
* **Frontend:** SvelteKit (SPA Mode), Svelte 5 (Runes exclusively: `$state`, `$derived`, `$effect`), TypeScript, Vite, Bun.
* **Backend:** Rust, Tauri 2.0 (OS-agnostic, cross-platform paths).
* **Async & Threading:** Tokio async runtime for I/O; dedicated, non-blocking background OS threads for the audio event loop.
* **Audio Engine:** `rodio` sink communicating strictly via lock-free `mpsc` channels.
* **Database Layer:** Embedded `rusqlite` for fast metadata caching and local index tracking.
* **Extension Sandbox:** Embedded Lua 5.4 (`mlua`) executing sandboxed runtime hooks with async `reqwest` HTTP capabilities.

## Build, Test & Lint Commands
- Backend Linting: `cargo clippy --all-targets -- -D warnings`
- Backend Footprint Check: `cargo size --release` or `cargo bloat`
- Frontend Dev Server: `bun run dev`
- Frontend Typing & Integrity Check: `bun run check`

## Guardrails & System Constraints (CRITICAL)

### Concurrency & Backend Performance
* **No Tokio Blocking:** `rusqlite` operations are strictly synchronous. NEVER execute database queries directly inside `async` Tauri commands. Wrap all DB calls in `tokio::task::spawn_blocking` or use a dedicated database actor thread.
* **Database Connection Strategy:** To prevent `SQLITE_BUSY` locks during background directory scanning, enforce a single-writer/multi-reader connection pool or funnel all writes through an `mpsc` channel to a dedicated DB thread.
* **Audio Thread Isolation:** The `rodio` sink and event loop must live on a dedicated OS thread outside the Tokio runtime. State mutation must occur purely via non-blocking `mpsc` message passing.

### Extension Sandbox (Lua) limits
* **Execution Bounding:** You must implement instruction count limits via `mlua`'s `set_hook` to prevent infinite loops.
* **Memory Bounding:** Implement strict memory allocation limits for the Lua environment. A rogue provider script must never compromise the 40MB application RAM budget.
* **Async Thread-Safety:** Ensure `reqwest` calls crossing the Rust-Lua boundary securely manage Tokio task lifetimes without leaking memory or panicking on dropped receivers.

### IPC Bridge & Frontend Hygiene
* **Throttle High-Frequency State:** Do not flood the Tauri IPC bridge. Throttle continuous updates (like track playback time) to a maximum of 4Hz, or rely on frontend interpolation via `requestAnimationFrame`.
* **Zero UI Interfiltration:** Keep the SvelteKit UI unstyled and purely structural. No large CSS frameworks (Tailwind is forbidden unless explicitly approved) or heavy JS animation libraries. Use Svelte 5 `$state` and `$derived` locally.
* **Primitive Marshalling:** Never stream large raw database vectors over `tauri::command`. Serialize into concise, paginated primitives. Map all Rust errors (`rusqlite::Error`, etc.) to explicit `Result<T, String>` messages before returning to the frontend.

### OS Integration
* **Desktop Media Controls:** Ensure integration with native OS media controls (MPRIS on Linux, SMTC on Windows, NowPlaying on macOS) using crates like `souvlaki`.

## Repository & Git Discipline
- Maintain complete commit cleanliness. Always squash feature branches before merging.
- Every commit generated must include proper attribution trailers:
  `Co-authored-by: Claude <noreply@anthropic.com>`

## Protected Directories (DO NOT EDIT)
- Auto-generated IPC state bindings or bridge interfaces.
- Upstream database schema migration histories.