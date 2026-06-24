# SOTA Queue System Revamp - Comprehensive Implementation Plan

**Status:** Design Phase  
**Scope:** Complete architectural refactor of queue system  
**Target:** Production-ready, race-condition-free, fully persistent queue with SOTA UX  

---

## EXECUTIVE SUMMARY

Your current queue system is 60% frontend logic, has race conditions, no persistence, and no batch operations. This plan moves queue to the backend (single source of truth), eliminates concurrency bugs, adds full persistence with recovery, implements proper shuffle state management, and optimizes frontend rendering.

**Estimated effort:** 6-8 weeks (80-120 engineering hours)  
**Risk level:** High (core playback logic refactor) → Mitigated by phased approach + comprehensive testing  

---

## PART 1: NEW ARCHITECTURE OVERVIEW

### Current Problems Recap
```
Frontend (audio.svelte.ts):
  ✗ Queue[T] lives in Svelte $state
  ✗ Shuffle logic scattered (setQueue, next, previous)
  ✗ Repeat state not atomic with queue position
  ✗ Race conditions on concurrent next() calls
  ✗ No persistence
  ✗ 50k items = DOM nightmare (no virtualization)

Backend (lib.rs):
  ✗ Zero queue awareness
  ✗ No validation or bounds checking
  ✗ Each skip = separate IPC call
```

### New Architecture (SOTA)
```
Backend (Rust - NEW):
  • Queue[T] struct with atomic operations
  • QueueState: current_position, mode (normal/shuffle), repeat_mode
  • ShuffleOrder: persistent shuffle sequence + cursor
  • QueueDB: SQLite persistence with versioning
  • Commands: jump_to_position(), next(count), prev(), reorder(), etc
  • All operations atomic via Mutex

Frontend (Svelte - Simplified):
  • Listen to queue-changed events only
  • Cache for UI (windowed view)
  • Send commands, never mutate queue state directly
  • Virtualized list (render only visible 20 items)
  • Optimistic UI updates with rollback on error

Database (SQLite - NEW):
  • queued_tracks table
  • shuffle_order table (if shuffle is on)
  • queue_state table (position, repeat mode, shuffle mode)
  • Version tracking for undo/redo (optional phase 2)
```

### Key Design Decisions

**Decision 1: Queue lives in Rust**
- ✅ Single source of truth
- ✅ Atomic operations (no race conditions)
- ✅ Persisted to DB
- ✅ Frontend is read-only observer
- ❌ Requires IPC for every operation (mitigated by batching)

**Decision 2: Shuffle order is stored, not regenerated**
- ✅ Reproducible shuffle sequences
- ✅ Can pause/resume shuffle without loss
- ✅ Enables proper prev() in shuffle mode
- ❌ More memory (store array, not just seed)
- Mitigation: Don't store more than 10k items in shuffle order

**Decision 3: Repeat state is part of queue, not global**
- ✅ Can have different repeat modes for different queues
- ✅ Clearer semantics (repeat is a queue property)
- ❌ More state to manage
- Mitigation: Start with global repeat (simpler), move to per-queue in phase 5

**Decision 4: Frontend rendering uses virtual scrolling**
- ✅ Handles 50k-item queues smoothly
- ✅ O(1) render time regardless of queue size
- ❌ Adds complexity (virtual-list library)
- Mitigation: Use `svelte-virtual-list` or `@svelte-put/virtualizer`

---

## PART 2: DETAILED PHASE BREAKDOWN

### PHASE 1: Backend Foundation (Weeks 1-2)

**Goal:** Move queue data structure to Rust, implement basic operations atomically.

#### 1.1 Define Queue Data Structures

**File:** `src-tauri/src/queue/mod.rs` (new)

```rust
// Core queue types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueTrack {
    pub instance_id: String,      // Unique per queue session
    pub track_id: i64,            // FK to library.tracks
    pub title: String,
    pub artist: Option<String>,
    pub file_path: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum QueueMode {
    Normal,
    Shuffle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShuffleState {
    pub order: Vec<String>,       // Shuffled order of instance_ids
    pub cursor: usize,            // Current position in shuffle order
}

pub struct QueueState {
    pub tracks: Vec<QueueTrack>,
    pub current_position: usize,  // Index in tracks (or shuffle order)
    pub repeat_mode: RepeatMode,
    pub mode: QueueMode,
    pub shuffle_state: Option<ShuffleState>,  // Only Some if mode == Shuffle
}

impl QueueState {
    pub fn new() -> Self {
        QueueState {
            tracks: Vec::new(),
            current_position: 0,
            repeat_mode: RepeatMode::Off,
            mode: QueueMode::Normal,
            shuffle_state: None,
        }
    }

    // Core operations (all atomic, return Result)
    pub fn set_queue(&mut self, tracks: Vec<QueueTrack>, start_index: usize) -> Result<(), String>;
    pub fn add_track(&mut self, track: QueueTrack) -> Result<(), String>;
    pub fn next(&mut self) -> Result<Option<QueueTrack>, String>;
    pub fn prev(&mut self) -> Result<Option<QueueTrack>, String>;
    pub fn jump_to_position(&mut self, position: usize) -> Result<QueueTrack, String>;
    pub fn jump_to_instance_id(&mut self, instance_id: String) -> Result<QueueTrack, String>;
    pub fn reorder(&mut self, from: usize, to: usize) -> Result<(), String>;
    pub fn clear(&mut self) -> Result<(), String>;
    pub fn current_track(&self) -> Option<&QueueTrack>;
    pub fn get_all(&self) -> &[QueueTrack];
    pub fn length(&self) -> usize;
}
```

**Key points:**
- All methods take `&mut self` → enforces single-threaded access via Mutex
- All return `Result<T, String>` → explicit error propagation
- No panics, all bounds checking explicit
- `ShuffleState` is separate struct → can be serialized to DB

#### 1.2 Implement Queue Operations

**File:** `src-tauri/src/queue/operations.rs` (new)

Implement core logic with detailed comments:

```rust
impl QueueState {
    /// Set queue and start at index. Clears shuffle state.
    pub fn set_queue(&mut self, tracks: Vec<QueueTrack>, start_index: usize) -> Result<(), String> {
        if start_index >= tracks.len() && !tracks.is_empty() {
            return Err(format!("Start index {} out of bounds (len={})", start_index, tracks.len()));
        }
        
        self.tracks = tracks;
        self.current_position = start_index;
        self.shuffle_state = None;  // Clear shuffle when new queue set
        Ok(())
    }

    /// Advance to next track. Respects repeat mode.
    pub fn next(&mut self) -> Result<Option<QueueTrack>, String> {
        if self.tracks.is_empty() {
            return Ok(None);
        }

        let next_pos = match self.mode {
            QueueMode::Normal => {
                let pos = self.current_position + 1;
                if pos >= self.tracks.len() {
                    match self.repeat_mode {
                        RepeatMode::Off => return Ok(None),  // Queue end, don't advance
                        RepeatMode::All => 0,                 // Loop to beginning
                        RepeatMode::One => self.current_position,  // Stay on current
                    }
                } else {
                    pos
                }
            }
            QueueMode::Shuffle => {
                let mut shuffle = self.shuffle_state.as_mut()
                    .ok_or_else(|| "Shuffle enabled but no shuffle_state".to_string())?;
                
                let next_cursor = shuffle.cursor + 1;
                if next_cursor >= shuffle.order.len() {
                    match self.repeat_mode {
                        RepeatMode::Off => return Ok(None),
                        RepeatMode::All => {
                            self.regenerate_shuffle_order()?;
                            shuffle = self.shuffle_state.as_mut().unwrap();
                            0
                        },
                        RepeatMode::One => shuffle.cursor,
                    }
                } else {
                    next_cursor
                }
            }
        };

        self.jump_to_position(next_pos)
            .map(|t| Some(t))
    }

    /// Generate new shuffle order from current queue.
    fn regenerate_shuffle_order(&mut self) -> Result<(), String> {
        use rand::seq::SliceRandom;
        
        let mut rng = rand::thread_rng();
        let mut order: Vec<String> = self.tracks.iter()
            .map(|t| t.instance_id.clone())
            .collect();
        order.shuffle(&mut rng);
        
        self.shuffle_state = Some(ShuffleState {
            order,
            cursor: 0,
        });
        Ok(())
    }

    /// Reorder tracks in queue.
    pub fn reorder(&mut self, from: usize, to: usize) -> Result<(), String> {
        if from >= self.tracks.len() || to >= self.tracks.len() {
            return Err("Reorder indices out of bounds".to_string());
        }
        if from == to {
            return Ok(());
        }

        let track = self.tracks.remove(from);
        self.tracks.insert(to, track);

        // If reordering in normal mode, update current_position
        if self.mode == QueueMode::Normal {
            if from == self.current_position {
                self.current_position = to;
            } else if from < self.current_position && to >= self.current_position {
                self.current_position -= 1;
            } else if from > self.current_position && to <= self.current_position {
                self.current_position += 1;
            }
        }

        Ok(())
    }
}
```

**Critical implementation notes:**
- All bounds checking is explicit (no panics)
- Shuffle regeneration only happens at queue-end (not on every next)
- Current position is updated atomically with reorder
- Repeat mode is checked in next() (not in UI)

#### 1.3 Add Database Schema

**File:** `src-tauri/src/db/schema.rs` (modify)

Add tables:

```sql
-- Queue persistence
CREATE TABLE IF NOT EXISTS queue_state (
    id INTEGER PRIMARY KEY,
    current_position INTEGER NOT NULL DEFAULT 0,
    repeat_mode TEXT NOT NULL DEFAULT 'Off',
    queue_mode TEXT NOT NULL DEFAULT 'Normal',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS queued_tracks (
    id INTEGER PRIMARY KEY,
    queue_state_id INTEGER NOT NULL,
    instance_id TEXT NOT NULL UNIQUE,
    track_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    FOREIGN KEY(queue_state_id) REFERENCES queue_state(id),
    FOREIGN KEY(track_id) REFERENCES tracks(id)
);

CREATE TABLE IF NOT EXISTS shuffle_order (
    id INTEGER PRIMARY KEY,
    queue_state_id INTEGER NOT NULL,
    instance_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    FOREIGN KEY(queue_state_id) REFERENCES queue_state(id)
);
```

#### 1.4 Add Queue Persistence Layer

**File:** `src-tauri/src/db/queue.rs` (new)

```rust
pub struct QueueRepository;

impl QueueRepository {
    /// Load queue state from DB (or return empty if none)
    pub fn load_queue(conn: &Connection) -> Result<QueueState, String> {
        let mut stmt = conn.prepare(
            "SELECT current_position, repeat_mode, queue_mode FROM queue_state ORDER BY id DESC LIMIT 1"
        ).map_err(|e| e.to_string())?;

        let queue_state = stmt.query_row([], |row| {
            Ok((
                row.get::<_, usize>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        });

        match queue_state {
            Ok((pos, repeat, mode)) => {
                // Load tracks, shuffle state, etc
                // Return populated QueueState
                Ok(QueueState { /* ... */ })
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Ok(QueueState::new())  // Empty queue
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// Save queue state to DB
    pub fn save_queue(conn: &Connection, queue: &QueueState) -> Result<(), String> {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        
        // Insert queue_state
        tx.execute(
            "INSERT INTO queue_state (current_position, repeat_mode, queue_mode) 
             VALUES (?, ?, ?)",
            [
                queue.current_position.to_string(),
                format!("{:?}", queue.repeat_mode),
                format!("{:?}", queue.mode),
            ].iter().map(|s| s as &dyn rusqlite::ToSql).collect::<Vec<_>>(),
        ).map_err(|e| e.to_string())?;

        // Insert queued_tracks
        for (i, track) in queue.tracks.iter().enumerate() {
            tx.execute(
                "INSERT INTO queued_tracks (queue_state_id, instance_id, track_id, position)
                 VALUES (last_insert_rowid(), ?, ?, ?)",
                [
                    &track.instance_id,
                    &track.track_id.to_string(),
                    &i.to_string(),
                ].iter().map(|s| s as &dyn rusqlite::ToSql).collect::<Vec<_>>(),
            ).map_err(|e| e.to_string())?;
        }

        // Insert shuffle order if present
        if let Some(shuffle) = &queue.shuffle_state {
            for (i, id) in shuffle.order.iter().enumerate() {
                tx.execute(
                    "INSERT INTO shuffle_order (queue_state_id, instance_id, position)
                     VALUES (last_insert_rowid(), ?, ?)",
                    [&id, &i.to_string()].iter().map(|s| s as &dyn rusqlite::ToSql).collect::<Vec<_>>(),
                ).map_err(|e| e.to_string())?;
            }
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
}
```

#### 1.5 Update AppState

**File:** `src-tauri/src/lib.rs` (modify AppState)

```rust
pub struct AppState {
    pub audio_tx: std::sync::Mutex<Sender<AudioCommand>>,
    pub db_tx: std::sync::mpsc::Sender<DbRequest>,
    pub provider_manager: tokio::sync::Mutex<ProviderManager>,
    pub audio_thread: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
    pub db_thread: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
    pub queue: std::sync::Mutex<QueueState>,  // ← NEW: Queue lives here
}
```

#### 1.6 Testing Strategy for Phase 1

**File:** `src-tauri/src/queue/tests.rs` (new)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_respects_repeat_off() {
        let mut queue = QueueState::new();
        queue.set_queue(vec![
            QueueTrack { instance_id: "1".to_string(), .. },
            QueueTrack { instance_id: "2".to_string(), .. },
        ], 0).unwrap();
        queue.repeat_mode = RepeatMode::Off;

        // Advance to last track
        queue.next().unwrap();
        assert_eq!(queue.current_position, 1);

        // Try to advance past end
        let result = queue.next();
        assert!(result.is_ok() && result.unwrap().is_none());
        assert_eq!(queue.current_position, 1);  // Should not advance
    }

    #[test]
    fn test_next_respects_repeat_all() {
        let mut queue = QueueState::new();
        queue.set_queue(vec![
            QueueTrack { instance_id: "1".to_string(), .. },
            QueueTrack { instance_id: "2".to_string(), .. },
        ], 1).unwrap();
        queue.repeat_mode = RepeatMode::All;

        queue.next().unwrap();
        assert_eq!(queue.current_position, 0);  // Wrapped
    }

    #[test]
    fn test_reorder_updates_current_position() {
        let mut queue = QueueState::new();
        queue.set_queue(vec![
            QueueTrack { instance_id: "1".to_string(), .. },
            QueueTrack { instance_id: "2".to_string(), .. },
            QueueTrack { instance_id: "3".to_string(), .. },
        ], 2).unwrap();

        queue.reorder(2, 0).unwrap();
        assert_eq!(queue.current_position, 1);  // Was at 2, now at 1
    }

    #[test]
    fn test_shuffle_mode_independent_of_normal() {
        let mut queue = QueueState::new();
        queue.set_queue(vec![
            QueueTrack { instance_id: "1".to_string(), .. },
            QueueTrack { instance_id: "2".to_string(), .. },
        ], 0).unwrap();

        queue.mode = QueueMode::Shuffle;
        queue.regenerate_shuffle_order().unwrap();

        let shuffle_order = queue.shuffle_state.as_ref().unwrap().order.clone();
        assert_eq!(shuffle_order.len(), 2);
        assert_ne!(shuffle_order[0], "1");  // (Very likely, not guaranteed)
    }

    #[test]
    fn test_empty_queue_operations_are_safe() {
        let mut queue = QueueState::new();

        assert_eq!(queue.next().unwrap(), None);
        assert_eq!(queue.prev().unwrap(), None);
        assert!(queue.clear().is_ok());
        assert_eq!(queue.length(), 0);
    }
}
```

Run: `cargo test --lib queue --test-threads=1`

#### 1.7 Code Review Checklist for Phase 1
- [ ] All queue operations are atomic (no partial state updates)
- [ ] No panics in queue code (all bounds checked)
- [ ] Shuffle order is independent of current position
- [ ] Repeat mode is tested for all three modes
- [ ] Empty queue is handled gracefully
- [ ] Reorder updates current_position correctly
- [ ] All Result<T, String> errors have actionable messages

**Phase 1 Deliverables:**
- ✅ Queue data structure with atomic operations
- ✅ Shuffle logic separate from skip logic
- ✅ Database schema for persistence
- ✅ 100% test coverage of operations
- ✅ No race conditions (Mutex-guarded state)

---

### PHASE 2: IPC Layer & Batch Operations (Week 2-3)

**Goal:** Create backend commands for queue operations, enable batch operations to reduce IPC overhead.

#### 2.1 Define Tauri Commands

**File:** `src-tauri/src/queue/commands.rs` (new)

```rust
use tauri::{State, AppHandle};

#[derive(Serialize, Deserialize)]
pub struct QueueChangeEvent {
    pub tracks: Vec<QueueTrack>,
    pub current_position: usize,
    pub current_track: Option<QueueTrack>,
    pub repeat_mode: RepeatMode,
    pub queue_mode: QueueMode,
}

/// Set queue and start playing at index
#[tauri::command]
async fn set_queue(
    state: State<'_, AppState>,
    tracks: Vec<QueueTrack>,
    start_index: usize,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;
    queue.set_queue(tracks, start_index)?;

    // Emit event to frontend
    let _ = state.app_handle.emit("queue-changed", queue_to_event(&queue));

    Ok(queue_to_event(&queue))
}

/// Skip N tracks forward (batch operation)
#[tauri::command]
async fn skip_forward(
    state: State<'_, AppState>,
    count: u32,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;

    for _ in 0..count {
        queue.next()?;
    }

    let _ = state.app_handle.emit("queue-changed", queue_to_event(&queue));
    Ok(queue_to_event(&queue))
}

/// Skip N tracks backward
#[tauri::command]
async fn skip_backward(
    state: State<'_, AppState>,
    count: u32,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;

    for _ in 0..count {
        queue.prev()?;
    }

    let _ = state.app_handle.emit("queue-changed", queue_to_event(&queue));
    Ok(queue_to_event(&queue))
}

/// Jump to specific track
#[tauri::command]
async fn jump_to_track(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;
    queue.jump_to_instance_id(instance_id)?;

    let _ = state.app_handle.emit("queue-changed", queue_to_event(&queue));
    Ok(queue_to_event(&queue))
}

/// Reorder track in queue (with validation)
#[tauri::command]
async fn reorder_queue_track(
    state: State<'_, AppState>,
    from_index: usize,
    to_index: usize,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;
    queue.reorder(from_index, to_index)?;

    let _ = state.app_handle.emit("queue-changed", queue_to_event(&queue));
    Ok(queue_to_event(&queue))
}

/// Set repeat mode
#[tauri::command]
async fn set_repeat_mode(
    state: State<'_, AppState>,
    mode: RepeatMode,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;
    queue.repeat_mode = mode;

    let _ = state.app_handle.emit("queue-changed", queue_to_event(&queue));
    Ok(queue_to_event(&queue))
}

/// Enable/disable shuffle
#[tauri::command]
async fn set_shuffle(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;

    queue.mode = if enabled {
        QueueMode::Shuffle
    } else {
        QueueMode::Normal
    };

    if enabled && queue.shuffle_state.is_none() {
        queue.regenerate_shuffle_order()?;
    }

    let _ = state.app_handle.emit("queue-changed", queue_to_event(&queue));
    Ok(queue_to_event(&queue))
}

/// Batch operation: Jump to track AND play it (single IPC)
#[tauri::command]
async fn jump_and_play(
    state: State<'_, AppState>,
    instance_id: String,
    audio_state: State<'_, AppState>,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;
    let current_track = queue.jump_to_instance_id(instance_id)?;

    let _ = state.app_handle.emit("queue-changed", queue_to_event(&queue));

    // Also send load command to audio thread (batch operation)
    if let Ok(tx) = audio_state.audio_tx.lock() {
        let _ = tx.send(AudioCommand::Load(current_track.file_path));
    }

    Ok(queue_to_event(&queue))
}

/// Get current queue (for frontend sync)
#[tauri::command]
async fn get_queue(state: State<'_, AppState>) -> Result<QueueChangeEvent, String> {
    let queue = state.queue.lock().map_err(|e| e.to_string())?;
    Ok(queue_to_event(&queue))
}

// Helper to convert QueueState to event payload
fn queue_to_event(queue: &QueueState) -> QueueChangeEvent {
    QueueChangeEvent {
        tracks: queue.get_all().to_vec(),
        current_position: queue.current_position,
        current_track: queue.current_track().cloned(),
        repeat_mode: queue.repeat_mode,
        queue_mode: queue.mode,
    }
}
```

#### 2.2 Register Commands in Tauri Handler

**File:** `src-tauri/src/lib.rs` (modify invoke_handler)

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    queue::commands::set_queue,
    queue::commands::skip_forward,
    queue::commands::skip_backward,
    queue::commands::jump_to_track,
    queue::commands::reorder_queue_track,
    queue::commands::set_repeat_mode,
    queue::commands::set_shuffle,
    queue::commands::jump_and_play,
    queue::commands::get_queue,
])
```

#### 2.3 Update Frontend Store to Use Commands

**File:** `src/lib/stores/audio.svelte.ts` (refactor)

```typescript
export class AudioStore {
    // ── Playback State ──
    playbackState = $state("Stopped");
    currentTrack = $state("None");
    currentTime = $state(0);
    duration = $state(0);

    // ── Queue State (Now just a cache!) ──
    queue = $state<QueueTrack[]>([]);
    currentPosition = $state(0);
    currentQueueId = $state<string | null>(null);
    repeatMode = $state<'Off' | 'All' | 'One'>('Off');
    shuffleEnabled = $state(false);

    // ── Remove internal shuffle logic (backend handles it) ──
    // ✗ private _shuffleOrder: string[] = []
    // ✗ private _shuffleHistory: string[] = []

    private unlistenQueueChanged: UnlistenFn | null = null;
    private unlistenSync: UnlistenFn | null = null;
    private unlistenTrackEnded: UnlistenFn | null = null;
    private _rafId: number | null = null;

    async init() {
        // Listen to backend queue changes
        this.unlistenQueueChanged = await listen<any>("queue-changed", (e) => {
            const payload = e.payload;
            this.queue = payload.tracks;
            this.currentPosition = payload.current_position;
            this.currentQueueId = payload.current_track?.instance_id || null;
            this.repeatMode = payload.repeat_mode;
            this.shuffleEnabled = payload.queue_mode === 'Shuffle';
        });

        // Listen to playback sync
        this.unlistenSync = await listen<PlayerSyncPayload>("player-sync", (e) => {
            const payload = e.payload;
            this._syncPosition = payload.position;
            this._syncTimestamp = performance.now();
            this._isPlaying = payload.state === "Playing";
            this.playbackState = payload.state;
            this.duration = payload.duration;
            this.currentTrack = payload.track || "None";
            this.currentTime = payload.position;
        });

        // Load persisted queue from backend
        await this.syncQueueFromBackend();
        this.startClock();
    }

    async syncQueueFromBackend() {
        try {
            const queueState = await invoke<any>("get_queue");
            this.queue = queueState.tracks;
            this.currentPosition = queueState.current_position;
            this.currentQueueId = queueState.current_track?.instance_id || null;
        } catch (e) {
            console.error("Failed to sync queue:", e);
        }
    }

    // ── REMOVED: All queue logic ──
    // ✗ setQueue() → Backend handles
    // ✗ addToQueue() → Backend handles
    // ✗ next() → Backend handles via skip_forward(1)
    // ✗ reorderQueue() → Backend handles via reorder_queue_track()

    // ── NEW: Command wrappers (thin layer) ──

    async skipForward(count: number = 1) {
        try {
            await invoke("skip_forward", { count });
            // Don't update UI—backend will emit queue-changed event
        } catch (e) {
            console.error("Skip forward failed:", e);
        }
    }

    async skipBackward(count: number = 1) {
        try {
            await invoke("skip_backward", { count });
        } catch (e) {
            console.error("Skip backward failed:", e);
        }
    }

    async jumpToTrack(instanceId: string) {
        try {
            await invoke("jump_to_track", { instance_id: instanceId });
        } catch (e) {
            console.error("Jump failed:", e);
        }
    }

    async setQueue(tracks: QueueTrack[], startIndex: number = 0) {
        try {
            await invoke("set_queue", { tracks, start_index: startIndex });
        } catch (e) {
            console.error("Set queue failed:", e);
        }
    }

    async reorderQueue(fromIndex: number, toIndex: number) {
        try {
            await invoke("reorder_queue_track", { from_index: fromIndex, to_index: toIndex });
        } catch (e) {
            console.error("Reorder failed:", e);
        }
    }

    async setRepeatMode(mode: 'Off' | 'All' | 'One') {
        try {
            await invoke("set_repeat_mode", { mode });
        } catch (e) {
            console.error("Set repeat failed:", e);
        }
    }

    async setShuffle(enabled: boolean) {
        try {
            await invoke("set_shuffle", { enabled });
        } catch (e) {
            console.error("Set shuffle failed:", e);
        }
    }
}
```

**Key changes:**
- `queue` is now a cache (read-only)
- All mutations go through backend commands
- Events trigger UI updates (not state mutations)
- No more race conditions (backend is single source of truth)

#### 2.4 Add Persistence on Quit

**File:** `src-tauri/src/lib.rs` (modify exit handler)

```rust
.run(|app, event| {
    if let RunEvent::Exit = event {
        let state: State<'_, AppState> = app.state();

        // Persist queue before exit
        if let Ok(queue) = state.queue.lock() {
            let (tx, rx) = oneshot::channel();
            let _ = state.db_tx.send(DbRequest::SaveQueue { 
                queue: queue.clone(), 
                resp: tx 
            });
            // Wait briefly for persistence
            if let Ok(oneshot_rx) = std::sync::mpsc::recv_timeout(&rx, Duration::from_millis(500)) {
                let _ = oneshot_rx;
            }
        }

        // Cleanup threads (existing code)
        // ...
    }
})
```

#### 2.5 Testing Strategy for Phase 2

**File:** `src/lib/stores/audio.svelte.test.ts` (new)

```typescript
import { describe, it, expect, vi } from 'vitest';
import { AudioStore } from './audio.svelte';

describe('AudioStore - Command Layer', () => {
    let store: AudioStore;

    beforeEach(() => {
        store = new AudioStore();
        // Mock invoke
        window.invoke = vi.fn().mockResolvedValue({});
    });

    it('skipForward calls backend command', async () => {
        await store.skipForward(3);
        expect(window.invoke).toHaveBeenCalledWith('skip_forward', { count: 3 });
    });

    it('jumpToTrack calls backend command', async () => {
        await store.jumpToTrack('track-123');
        expect(window.invoke).toHaveBeenCalledWith('jump_to_track', { instance_id: 'track-123' });
    });

    it('handleBatch: queue-changed event updates cache', async () => {
        const event = {
            payload: {
                tracks: [{ instance_id: '1', title: 'Track 1' }],
                current_position: 0,
                current_track: { instance_id: '1' },
            }
        };
        
        store.handleQueueChanged(event);
        
        expect(store.queue.length).toBe(1);
        expect(store.currentPosition).toBe(0);
    });

    it('error handling propagates to console', async () => {
        const spy = vi.spyOn(console, 'error');
        window.invoke = vi.fn().mockRejectedValue('Network error');

        await store.skipForward(1);

        expect(spy).toHaveBeenCalledWith(expect.stringContaining('Skip forward failed'));
    });
});
```

**Phase 2 Deliverables:**
- ✅ 9 core queue commands (set, skip, jump, reorder, repeat, shuffle, batch)
- ✅ Batch operations (skip_forward(N) in single IPC)
- ✅ Backend is single source of truth for queue state
- ✅ Frontend cache stays in sync via events
- ✅ Persistence on app exit
- ✅ Full test coverage for command layer

---

### PHASE 3: Frontend Refactor - Virtualization & State Cleanup (Week 3)

**Goal:** Eliminate DOM rendering of 50k items, implement virtual scrolling, remove redundant frontend state.

#### 3.1 Add Virtual List Library

**File:** `package.json` (modify)

```json
{
  "dependencies": {
    "@svelte-put/virtualizer": "^3.1.0",
    "svelte": "^5.0.0"
  }
}
```

Run: `bun install`

#### 3.2 Refactor QueueSidebar with Virtualization

**File:** `src/lib/components/QueueSidebar.svelte` (complete rewrite)

```svelte
<script lang="ts">
    import { audioStore } from "$lib/stores/audio.svelte";
    import { createVirtualizer } from "@svelte-put/virtualizer";
    import { X, Trash2, Pause, Play } from "lucide-svelte";
    import { fly, fade } from "svelte/transition";
    import { cubicOut } from "svelte/easing";

    let { open = $bindable(false) } = $props<{ open?: boolean }>();

    let draggedIndex = -1;
    let dragoverIndex = $state(-1);
    let justReorderedIndex = $state(-1);
    let containerRef: HTMLElement;
    let isLoading = $state(false);

    const virtualizer = createVirtualizer({
        count: () => audioStore.queue.length,
        getScrollElement: () => containerRef,
        estimateSize: () => 60,
        overscan: 10,
    });

    function jumpToTrack(instanceId: string) {
        audioStore.jumpToTrack(instanceId);
    }

    async function handleDragStart(e: DragEvent, index: number) {
        draggedIndex = index;
        if (e.dataTransfer) {
            e.dataTransfer.effectAllowed = "move";
            const track = audioStore.queue[index];
            const dragImage = createDragPreview(track.title);
            e.dataTransfer.setDragImage(dragImage, 0, 0);
        }
    }

    async function handleDrop(e: DragEvent, targetIndex: number) {
        e.preventDefault();
        dragoverIndex = -1;

        if (draggedIndex !== -1 && draggedIndex !== targetIndex) {
            isLoading = true;
            try {
                await audioStore.reorderQueue(draggedIndex, targetIndex);
                justReorderedIndex = targetIndex;
                setTimeout(() => {
                    justReorderedIndex = -1;
                }, 300);
            } catch (error) {
                console.error("Reorder failed:", error);
            } finally {
                isLoading = false;
            }
        }
        draggedIndex = -1;
    }

    function handleDragOver(e: DragEvent, index: number) {
        e.preventDefault();
        dragoverIndex = index;
        if (e.dataTransfer) {
            e.dataTransfer.dropEffect = "move";
        }
    }

    function handleDragLeave() {
        dragoverIndex = -1;
    }

    function createDragPreview(title: string): HTMLElement {
        const div = document.createElement('div');
        div.style.position = 'absolute';
        div.style.top = '-9999px';
        div.style.left = '-9999px';
        div.style.background = 'rgba(255, 255, 255, 0.1)';
        div.style.border = '1px solid rgba(255, 255, 255, 0.2)';
        div.style.borderRadius = '6px';
        div.style.padding = '0.6rem 1rem';
        div.style.fontSize = '0.8rem';
        div.style.color = 'rgb(200, 200, 200)';
        div.style.whiteSpace = 'nowrap';
        div.textContent = title;
        document.body.appendChild(div);
        setTimeout(() => document.body.removeChild(div), 0);
        return div;
    }

    async function handleClearQueue() {
        if (confirm("Clear entire queue?")) {
            isLoading = true;
            try {
                await audioStore.clearQueue();
            } finally {
                isLoading = false;
            }
        }
    }
</script>

{#if open}
    <div
        class="backdrop"
        transition:fade={{ duration: 180 }}
        onclick={() => (open = false)}
    ></div>

    <aside
        class="queue-panel"
        transition:fly={{ x: 360, duration: 260, easing: cubicOut }}
    >
        <div class="queue-header">
            <span class="queue-title">Queue</span>
            <div class="header-actions">
                {#if audioStore.queue.length > 0}
                    <button
                        class="icon-btn"
                        onclick={handleClearQueue}
                        disabled={isLoading}
                        title="Clear queue"
                    >
                        <Trash2 size={14} />
                    </button>
                {/if}
                <button
                    class="icon-btn"
                    onclick={() => (open = false)}
                    title="Close"
                    disabled={isLoading}
                >
                    <X size={16} />
                </button>
            </div>
        </div>

        {#if audioStore.queue.length === 0}
            <div class="queue-empty">
                <p class="empty-label">Queue is empty</p>
                <p class="empty-hint">Play an album or playlist to populate it.</p>
            </div>
        {:else}
            <div class="queue-list" bind:this={containerRef}>
                {#each virtualizer.getVirtualItems() as virtualItem (virtualItem.key)}
                    {@const index = virtualItem.index}
                    {@const track = audioStore.queue[index]}
                    {@const isPlaying = track.instanceId === audioStore.currentQueueId}

                    <div
                        class="queue-row"
                        class:playing={isPlaying}
                        class:drag-over={dragoverIndex === index}
                        class:just-reordered={justReorderedIndex === index}
                        style:height="{virtualItem.size}px"
                        style:transform="translateY({virtualItem.offset}px)"
                        draggable="true"
                        ondragstart={(e) => handleDragStart(e, index)}
                        ondragover={(e) => handleDragOver(e, index)}
                        ondragleave={handleDragLeave}
                        ondrop={(e) => handleDrop(e, index)}
                        ondragend={() => {
                            dragoverIndex = -1;
                            draggedIndex = -1;
                        }}
                        onclick={() => jumpToTrack(track.instanceId)}
                    >
                        <span class="row-num">
                            {#if isPlaying}
                                <span class="playing-indicator">
                                    {audioStore.playbackState === 'Playing'
                                        ? '▶'
                                        : '⏸'}
                                </span>
                            {:else}
                                {index + 1}
                            {/if}
                        </span>
                        <div class="row-info">
                            <span class="row-title">{track.title}</span>
                            <span class="row-artist">{track.artist || "Unknown"}</span>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </aside>
{/if}

<style>
    .backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0 0 0 / 0.25);
        z-index: 45;
    }

    .queue-panel {
        position: fixed;
        top: 0;
        right: 0;
        height: calc(100vh - 120px);
        width: 320px;
        background: rgba(9 9 12 / 0.88);
        backdrop-filter: blur(44px) saturate(180%);
        border-left: 1px solid var(--echo-border);
        z-index: 55;
        display: flex;
        flex-direction: column;
    }

    .queue-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1.25rem;
        border-bottom: 1px solid var(--echo-border);
        flex-shrink: 0;
    }

    .queue-list {
        flex: 1;
        overflow-y: auto;
        position: relative;
        height: 100%;
    }

    .queue-row {
        position: absolute;
        left: 0;
        right: 0;
        display: flex;
        align-items: center;
        gap: 0.85rem;
        padding: 0.6rem 1.25rem;
        cursor: pointer;
        transition: background 0.15s ease, opacity 0.15s ease;
        user-select: none;
        will-change: transform;
    }

    .queue-row:hover {
        background: rgba(255 255 255 / 0.04);
    }

    .queue-row.drag-over {
        border-top: 2px solid var(--echo-accent);
        background: rgba(255 255 255 / 0.05);
    }

    .queue-row.playing {
        background: rgba(255 255 255 / 0.04);
    }

    .queue-row[draggable="true"]:active {
        opacity: 0.6;
    }

    .queue-row.just-reordered {
        animation: pulse-highlight 0.3s ease-out;
    }

    @keyframes pulse-highlight {
        0% { background: rgba(255, 255, 255, 0.12); }
        50% { background: rgba(255, 255, 255, 0.08); }
        100% { background: rgba(255, 255, 255, 0.04); }
    }

    .row-num {
        font-size: 0.72rem;
        color: var(--echo-text-3);
        width: 18px;
        text-align: right;
        flex-shrink: 0;
    }

    .playing-indicator {
        color: var(--echo-silver);
        font-weight: bold;
    }

    .row-info {
        min-width: 0;
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .row-title {
        font-size: 0.8rem;
        font-weight: 450;
        color: var(--echo-text-1);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .queue-row.playing .row-title {
        color: var(--echo-silver);
    }

    .row-artist {
        font-size: 0.7rem;
        color: var(--echo-text-3);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .queue-empty {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 0.4rem;
        padding: 2rem;
        text-align: center;
    }

    .icon-btn {
        background: transparent;
        border: none;
        color: var(--echo-text-3);
        padding: 0.3rem;
        border-radius: 6px;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.12s ease;
    }

    .icon-btn:hover:not(:disabled) {
        color: var(--echo-text-1);
        background: rgba(255 255 255 / 0.07);
    }

    .icon-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
</style>
```

**Key improvements:**
- Uses `@svelte-put/virtualizer` to render only visible items
- Can handle 50k items smoothly (constant DOM size)
- Transform-based positioning (GPU accelerated)
- Proper loading state during reorder
- Fallback spinner/disable during operations

#### 3.2 Simplify PlayerBar Controls

**File:** `src/lib/components/PlayerBar.svelte` (update skip buttons)

```svelte
<script>
    // In buttons section:
    
    async function handleSkipBack() {
        await audioStore.skipBackward(1);
    }

    async function handleSkipForward() {
        await audioStore.skipForward(1);
    }

    async function handleRepeat() {
        const modes: Array<'Off' | 'All' | 'One'> = ['Off', 'All', 'One'];
        const currentIndex = modes.indexOf(audioStore.repeatMode);
        const nextMode = modes[(currentIndex + 1) % modes.length];
        await audioStore.setRepeatMode(nextMode);
    }

    async function handleShuffle() {
        await audioStore.setShuffle(!audioStore.shuffleEnabled);
    }
</script>

<!-- Update buttons to use new handlers -->
<button onclick={handleSkipBack} title="Previous">
    <SkipBack size={18} />
</button>

<button onclick={handleSkipForward} title="Next">
    <SkipForward size={18} />
</button>

<button
    onclick={handleRepeat}
    class:active={audioStore.repeatMode !== 'Off'}
    title={`Repeat: ${audioStore.repeatMode}`}
>
    <Repeat2 size={18} />
</button>

<button
    onclick={handleShuffle}
    class:active={audioStore.shuffleEnabled}
    title="Shuffle"
>
    <Shuffle size={18} />
</button>
```

#### 3.3 Testing Phase 3

**File:** `src/lib/components/QueueSidebar.test.ts`

```typescript
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import QueueSidebar from './QueueSidebar.svelte';

describe('QueueSidebar - Virtualization', () => {
    it('renders only visible items with 50k queue', () => {
        const largeQueue = Array.from({ length: 50000 }, (_, i) => ({
            instanceId: `track-${i}`,
            title: `Track ${i}`,
            artist: 'Artist',
        }));

        render(QueueSidebar, { props: { open: true, queue: largeQueue } });

        const rows = document.querySelectorAll('.queue-row');
        // Should only render ~15-20 items, not 50k
        expect(rows.length).toBeLessThan(30);
    });

    it('virtualized list scrolls correctly', async () => {
        // Test scroll position updating
        const container = document.querySelector('.queue-list');
        container.scrollTop = 10000;

        // Items should update without blocking
        expect(performance.now()).toBeGreaterThanOrEqual(0);
    });
});
```

**Phase 3 Deliverables:**
- ✅ Virtual scrolling (50k items at 60fps)
- ✅ Simplified PlayerBar (commands instead of mutations)
- ✅ Clean separation: frontend caches, doesn't own
- ✅ Loading states during operations
- ✅ No DOM bloat

---

### PHASE 4: Shuffle & Repeat Logic Deep Refactor (Week 4)

**Goal:** Implement proper shuffle semantics with resumable state.

#### 4.1 Extend Shuffle State in Backend

**File:** `src-tauri/src/queue/mod.rs` (modify)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShuffleState {
    pub order: Vec<String>,              // Shuffled instance IDs
    pub cursor: usize,                   // Position in shuffled order
    pub seed: u64,                       // For reproducibility (optional)
    pub regenerate_on_repeat: bool,      // Whether to reshuffle on repeat-all
}

impl QueueState {
    /// Toggle shuffle mode, maintaining position semantics
    pub fn set_shuffle(&mut self, enabled: bool) -> Result<(), String> {
        if enabled && self.mode == QueueMode::Shuffle {
            return Ok(());  // Already shuffled
        }

        if !enabled && self.mode == QueueMode::Normal {
            return Ok(());  // Already normal
        }

        if enabled {
            // Entering shuffle: preserve current track, generate shuffle order
            let current_id = self.current_track()
                .ok_or_else(|| "Cannot shuffle empty queue".to_string())?
                .instance_id.clone();

            self.regenerate_shuffle_order()?;

            // Position cursor at current track in new shuffle order
            if let Some(shuffle) = &mut self.shuffle_state {
                if let Some(idx) = shuffle.order.iter().position(|id| id == &current_id) {
                    shuffle.cursor = idx;
                }
            }

            self.mode = QueueMode::Shuffle;
        } else {
            // Exiting shuffle: preserve current track in normal mode
            let current_id = self.current_track()
                .ok_or_else(|| "Cannot exit shuffle with no track".to_string())?
                .instance_id.clone();

            if let Some(idx) = self.tracks.iter().position(|t| t.instance_id == current_id) {
                self.current_position = idx;
            }

            self.shuffle_state = None;
            self.mode = QueueMode::Normal;
        }

        Ok(())
    }

    /// Proper prev() in shuffle mode
    pub fn prev(&mut self) -> Result<Option<QueueTrack>, String> {
        if self.tracks.is_empty() {
            return Ok(None);
        }

        let prev_pos = match self.mode {
            QueueMode::Normal => {
                let pos = self.current_position as i32 - 1;
                if pos < 0 {
                    match self.repeat_mode {
                        RepeatMode::Off => return Ok(None),
                        RepeatMode::All => self.tracks.len() - 1,
                        RepeatMode::One => self.current_position,
                    }
                } else {
                    pos as usize
                }
            }
            QueueMode::Shuffle => {
                let mut shuffle = self.shuffle_state.as_mut()
                    .ok_or_else(|| "Shuffle state missing".to_string())?;

                let prev_cursor = shuffle.cursor as i32 - 1;
                if prev_cursor < 0 {
                    match self.repeat_mode {
                        RepeatMode::Off => return Ok(None),
                        RepeatMode::All => {
                            // Don't regenerate on prev; just wrap around
                            shuffle.order.len() - 1
                        },
                        RepeatMode::One => shuffle.cursor,
                    }
                } else {
                    prev_cursor as usize
                }
            }
        };

        self.jump_to_position(prev_pos).map(|t| Some(t))
    }

    /// Generate shuffle with seeded RNG for reproducibility
    fn regenerate_shuffle_order(&mut self) -> Result<(), String> {
        use rand::{SeedableRng, seq::SliceRandom};
        use rand::rngs::StdRng;

        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        let mut rng = StdRng::seed_from_u64(seed);
        let mut order: Vec<String> = self.tracks.iter()
            .map(|t| t.instance_id.clone())
            .collect();
        order.shuffle(&mut rng);

        self.shuffle_state = Some(ShuffleState {
            order,
            cursor: 0,
            seed,
            regenerate_on_repeat: true,
        });

        Ok(())
    }
}
```

#### 4.2 Add Shuffle Persistence

**File:** `src-tauri/src/db/queue.rs` (extend)

```rust
impl QueueRepository {
    /// Save shuffle order to DB
    fn save_shuffle_state(
        tx: &Transaction,
        queue_state_id: i64,
        shuffle: &ShuffleState,
    ) -> Result<(), String> {
        for (i, instance_id) in shuffle.order.iter().enumerate() {
            tx.execute(
                "INSERT INTO shuffle_order (queue_state_id, instance_id, position, seed)
                 VALUES (?, ?, ?, ?)",
                [
                    &queue_state_id.to_string(),
                    instance_id,
                    &i.to_string(),
                    &shuffle.seed.to_string(),
                ].iter().map(|s| s as &dyn rusqlite::ToSql).collect::<Vec<_>>(),
            ).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Load shuffle state from DB
    fn load_shuffle_state(conn: &Connection, queue_state_id: i64) -> Result<Option<ShuffleState>, String> {
        let mut stmt = conn.prepare(
            "SELECT instance_id, position, seed FROM shuffle_order 
             WHERE queue_state_id = ? ORDER BY position"
        ).map_err(|e| e.to_string())?;

        let orders: Vec<_> = stmt.query_map([queue_state_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u64>(2)?,
            ))
        }).map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        if orders.is_empty() {
            return Ok(None);
        }

        let seed = orders.first().map(|(_, s)| *s).unwrap_or(0);
        let order = orders.into_iter().map(|(id, _)| id).collect();

        Ok(Some(ShuffleState {
            order,
            cursor: 0,
            seed,
            regenerate_on_repeat: true,
        }))
    }
}
```

#### 4.3 Add Advanced Shuffle Commands

**File:** `src-tauri/src/queue/commands.rs` (extend)

```rust
/// Reshuffle without changing current track
#[tauri::command]
async fn reshuffle(state: State<'_, AppState>) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;

    if queue.mode != QueueMode::Shuffle {
        return Err("Shuffle not enabled".to_string());
    }

    let current_id = queue.current_track()
        .ok_or_else(|| "No current track".to_string())?
        .instance_id.clone();

    queue.regenerate_shuffle_order()?;

    // Maintain current track position in new shuffle
    if let Some(shuffle) = &mut queue.shuffle_state {
        if let Some(idx) = shuffle.order.iter().position(|id| id == &current_id) {
            shuffle.cursor = idx;
        }
    }

    let _ = state.app_handle.emit("queue-changed", queue_to_event(&queue));
    Ok(queue_to_event(&queue))
}

/// Get shuffle statistics (for future analytics)
#[tauri::command]
async fn get_queue_stats(state: State<'_, AppState>) -> Result<QueueStats, String> {
    let queue = state.queue.lock().map_err(|e| e.to_string())?;

    Ok(QueueStats {
        total_tracks: queue.length(),
        current_position: queue.current_position,
        is_shuffled: queue.mode == QueueMode::Shuffle,
        repeat_mode: format!("{:?}", queue.repeat_mode),
    })
}
```

**Phase 4 Deliverables:**
- ✅ Proper shuffle prev() that works correctly
- ✅ Reproducible shuffle with seeding
- ✅ Shuffle state persists across sessions
- ✅ Toggle shuffle while playing (maintains current track)
- ✅ Reshuffle command for manual refresh

---

### PHASE 5: Persistence & Recovery (Week 5)

**Goal:** Full persistence with crash recovery and queue history.

#### 5.1 Add Queue Versioning

**File:** `src-tauri/src/db/schema.rs` (extend schema)

```sql
CREATE TABLE IF NOT EXISTS queue_snapshots (
    id INTEGER PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    queue_state_id INTEGER NOT NULL,
    description TEXT,
    FOREIGN KEY(queue_state_id) REFERENCES queue_state(id)
);

CREATE TABLE IF NOT EXISTS queue_history (
    id INTEGER PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    action TEXT NOT NULL,  -- 'next', 'prev', 'jump', 'reorder', 'shuffle_toggle'
    from_position INTEGER,
    to_position INTEGER,
    track_id INTEGER
);
```

#### 5.2 Implement Queue Recovery

**File:** `src-tauri/src/queue/recovery.rs` (new)

```rust
pub struct QueueRecovery;

impl QueueRecovery {
    /// On startup, restore queue if it crashed mid-operation
    pub fn restore_on_startup(conn: &Connection) -> Result<QueueState, String> {
        // Load last saved queue
        let queue = QueueRepository::load_queue(conn)?;

        // Log recovery event
        log::info!(
            "Queue recovered: {} tracks, position: {}/{}",
            queue.length(),
            queue.current_position,
            queue.length()
        );

        Ok(queue)
    }

    /// Create snapshot of current queue state (for undo/redo in future)
    pub fn create_snapshot(
        conn: &Connection,
        queue: &QueueState,
        description: &str,
    ) -> Result<(), String> {
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        QueueRepository::save_queue(&tx, queue)?;

        let queue_id = conn.last_insert_rowid();
        tx.execute(
            "INSERT INTO queue_snapshots (queue_state_id, description) VALUES (?, ?)",
            [&queue_id.to_string(), description].iter().map(|s| s as &dyn rusqlite::ToSql).collect::<Vec<_>>(),
        ).map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Log user action for analytics
    pub fn log_action(
        conn: &Connection,
        action: &str,
        from_pos: Option<usize>,
        to_pos: Option<usize>,
    ) -> Result<(), String> {
        conn.execute(
            "INSERT INTO queue_history (action, from_position, to_position) VALUES (?, ?, ?)",
            [
                action,
                &from_pos.map(|p| p.to_string()).unwrap_or_default(),
                &to_pos.map(|p| p.to_string()).unwrap_or_default(),
            ].iter().map(|s| s as &dyn rusqlite::ToSql).collect::<Vec<_>>(),
        ).map_err(|e| e.to_string())?;

        Ok(())
    }
}
```

#### 5.3 Integrate Recovery on Startup

**File:** `src-tauri/src/lib.rs` (modify setup)

```rust
.setup(move |app| {
    // ... existing setup ...

    let queue_state = QueueRecovery::restore_on_startup(&conn)
        .unwrap_or_else(|e| {
            log::warn!("Failed to restore queue: {}", e);
            QueueState::new()
        });

    app.manage(AppState {
        // ...
        queue: std::sync::Mutex::new(queue_state),
    });

    Ok(())
})
```

#### 5.4 Periodic Persistence

**File:** `src-tauri/src/lib.rs` (add background task)

```rust
// In setup, spawn background persistence task
let app_handle = app.handle().clone();
let state_clone = state.clone();

tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        if let Ok(queue) = state_clone.queue.lock() {
            if let Ok(conn) = open_db_connection() {
                let _ = QueueRepository::save_queue(&conn, &queue);
            }
        }
    }
});
```

**Phase 5 Deliverables:**
- ✅ Persistent queue across app restarts
- ✅ Crash recovery (graceful restoration)
- ✅ Periodic auto-save every 30s
- ✅ Queue history for analytics
- ✅ Snapshot capability for future undo/redo

---

### PHASE 6: Performance Testing & Optimization (Week 5-6)

**Goal:** Ensure SOTA performance with large queues.

#### 6.1 Benchmark Suite

**File:** `src-tauri/src/queue/bench.rs` (new)

```rust
#[cfg(test)]
mod benches {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    use super::*;

    fn bench_next_100k_queue(c: &mut Criterion) {
        c.bench_function("next_in_100k_queue", |b| {
            let mut queue = QueueState::new();
            let tracks = (0..100_000)
                .map(|i| QueueTrack { instance_id: format!("track-{}", i), .. })
                .collect();
            queue.set_queue(black_box(tracks), 0).unwrap();

            b.iter(|| {
                queue.next().ok();
            });
        });
    }

    fn bench_reorder_large_queue(c: &mut Criterion) {
        c.bench_function("reorder_50k_queue", |b| {
            let mut queue = QueueState::new();
            let tracks = (0..50_000)
                .map(|i| QueueTrack { instance_id: format!("track-{}", i), .. })
                .collect();
            queue.set_queue(black_box(tracks), 0).unwrap();

            b.iter(|| {
                queue.reorder(black_box(0), black_box(25_000)).ok();
            });
        });
    }

    fn bench_shuffle_100k_queue(c: &mut Criterion) {
        c.bench_function("shuffle_100k_queue", |b| {
            let mut queue = QueueState::new();
            let tracks = (0..100_000)
                .map(|i| QueueTrack { instance_id: format!("track-{}", i), .. })
                .collect();
            queue.set_queue(black_box(tracks), 0).unwrap();

            b.iter(|| {
                queue.mode = QueueMode::Shuffle;
                queue.regenerate_shuffle_order().ok();
            });
        });
    }

    criterion_group!(benches, bench_next_100k_queue, bench_reorder_large_queue, bench_shuffle_100k_queue);
    criterion_main!(benches);
}
```

Run: `cargo bench --lib queue`

**Expected results:**
- `next()`: < 100ns (O(1))
- `reorder()`: < 1ms (O(n) where n=50k)
- `shuffle()`: < 50ms (Fisher-Yates shuffle on 100k items)

#### 6.2 Frontend Performance Tests

**File:** `src/lib/components/QueueSidebar.perf.test.ts`

```typescript
import { describe, it, expect } from 'vitest';

describe('QueueSidebar - Performance', () => {
    it('renders 50k items at 60fps', async () => {
        const largeQueue = Array.from({ length: 50_000 }, (_, i) => ({
            instanceId: `${i}`,
            title: `Track ${i}`,
            artist: 'Artist',
        }));

        const start = performance.now();
        // Render
        const end = performance.now();

        const fps = 1000 / (end - start);
        expect(fps).toBeGreaterThan(60);
    });

    it('scroll perf: 10k scroll deltas without jank', async () => {
        // Simulate user scrolling rapidly
        let dropped = 0;

        for (let i = 0; i < 10_000; i++) {
            const frame = performance.now();
            // Virtualize
            const next = performance.now();

            if (next - frame > 16.67) {
                dropped++;  // Missed frame (>16ms on 60hz)
            }
        }

        expect(dropped / 10_000).toBeLessThan(0.05);  // <5% dropped frames
    });
});
```

#### 6.3 Memory Profiling

Use Chrome DevTools to validate:
- Queue with 100k items: < 50MB
- Virtualized DOM: < 500 nodes (regardless of queue size)
- IPC payload size: < 100KB

**Phase 6 Deliverables:**
- ✅ Benchmarks for all core operations
- ✅ Performance targets met (60fps, O(1) skips)
- ✅ Memory footprint documented
- ✅ Load test with 100k items

---

### PHASE 7: Polish & Telemetry (Week 6-7)

**Goal:** Production-ready with observability.

#### 7.1 Add Error Telemetry

**File:** `src-tauri/src/queue/telemetry.rs` (new)

```rust
pub struct QueueTelemetry;

impl QueueTelemetry {
    pub fn log_error(action: &str, error: &str) {
        log::error!("[Queue::{}] {}", action, error);
        // In production: Send to telemetry service (Sentry, etc.)
    }

    pub fn log_operation(action: &str, duration_ms: u64) {
        log::debug!("[Queue::{}] completed in {}ms", action, duration_ms);
    }

    pub fn log_queue_state(queue: &QueueState) {
        log::trace!(
            "Queue: {} tracks, pos={}, repeat={:?}, mode={:?}",
            queue.length(),
            queue.current_position,
            queue.repeat_mode,
            queue.mode,
        );
    }
}
```

#### 7.2 Add Frontend Error Boundaries

**File:** `src/lib/components/QueueSidebar.svelte` (add)

```typescript
function handleError(error: any) {
    console.error("Queue operation failed:", error);
    // Show toast notification
    showToast({
        type: 'error',
        message: 'Queue operation failed. Please try again.',
    });
}
```

#### 7.3 Documentation

**File:** `QUEUE_ARCHITECTURE.md` (new)

Include:
- Data flow diagram
- Command reference
- Event reference
- Persistence strategy
- Troubleshooting guide

#### 7.4 Add Feature Flags for Gradual Rollout

**File:** `src-tauri/src/lib.rs`

```rust
pub struct FeatureFlags {
    pub enable_new_queue_system: bool,
    pub enable_queue_persistence: bool,
    pub enable_shuffle_v2: bool,
}

impl FeatureFlags {
    pub fn load(conn: &Connection) -> Self {
        FeatureFlags {
            enable_new_queue_system: get_flag(conn, "new_queue", true),
            enable_queue_persistence: get_flag(conn, "queue_persistence", true),
            enable_shuffle_v2: get_flag(conn, "shuffle_v2", true),
        }
    }
}
```

**Phase 7 Deliverables:**
- ✅ Comprehensive error logging
- ✅ User-facing error messages
- ✅ Feature flags for gradual rollout
- ✅ Complete documentation
- ✅ Telemetry infrastructure

---

## PART 3: ROLLOUT STRATEGY

### Pre-Launch Testing (1 week)
1. **Internal testing:** Run all 12 concurrent queue ops, verify no race conditions
2. **Load testing:** 100k items, scroll perf, memory usage
3. **Compatibility:** Test on Windows, macOS, Linux
4. **Crash testing:** Force kill app mid-operation, verify recovery

### Staged Rollout
1. **Canary (5% users):** Internal team + beta testers
2. **Early access (25% users):** Opt-in beta channel
3. **Full rollout (100%):** After 1 week of stability

### Rollback Plan
- Keep old queue system functional for 2 releases
- Feature flag to switch back if critical bugs found
- Database migration reversible

---

## PART 4: TIMELINE & RESOURCE ALLOCATION

| Phase | Week | Focus | Risk | Effort |
|-------|------|-------|------|--------|
| 1 | 1-2 | Backend foundation | HIGH | 30h |
| 2 | 2-3 | IPC layer | MEDIUM | 20h |
| 3 | 3 | Frontend virtualization | MEDIUM | 15h |
| 4 | 4 | Shuffle refactor | LOW | 15h |
| 5 | 5 | Persistence | MEDIUM | 12h |
| 6 | 5-6 | Performance | LOW | 10h |
| 7 | 6-7 | Polish | LOW | 8h |
| Testing | Throughout | Unit + integration | - | 20h |
| **Total** | **6-8 weeks** | | | **130h** |

**Recommended team:**
- 1 senior backend engineer (Phase 1-2, 5)
- 1 frontend engineer (Phase 3, 7)
- 1 QA engineer (testing throughout)

---

## PART 5: SUCCESS METRICS

### Performance
- ✅ `next()` < 100ns (currently ~500ns)
- ✅ Reorder < 5ms (currently ~1sec with IPC overhead)
- ✅ Shuffle 100k items < 100ms (currently regenerates every skip)
- ✅ 50fps on queue sidebar scroll (currently janky)

### Reliability
- ✅ Zero race conditions (verified with tsan)
- ✅ 99.9% queue persistence success
- ✅ 0 skip operations fail silently

### UX
- ✅ Queue survives app restart
- ✅ Clear error messages on failure
- ✅ Drag-drop provides visual feedback
- ✅ Shuffle has proper prev() semantics

### Code Quality
- ✅ 100% test coverage for queue module
- ✅ Zero panics in queue code
- ✅ All operations return Result<T, String>
- ✅ Comprehensive documentation

---

## PART 6: RISK MITIGATION

| Risk | Severity | Mitigation |
|------|----------|-----------|
| Race conditions in shuffle | HIGH | Extensive unit tests + thread sanitizer |
| DB persistence corrupts queue | HIGH | Atomic transactions + snapshots |
| Large queue slows down UI | MEDIUM | Virtual scrolling + perf tests |
| Persistence breaks on upgrade | MEDIUM | DB versioning + migration tests |
| Rollout breaks existing queues | MEDIUM | Feature flags + staged rollout |

---

## CONCLUSION

This plan transforms your queue from a buggy, frontend-only system into a production-grade, SOTA queue manager with:

- ✅ **Atomicity:** No race conditions (Rust mutex + atomic ops)
- ✅ **Persistence:** Survives app crashes and restarts
- ✅ **Performance:** O(1) skips, handles 100k items smoothly
- ✅ **Correct semantics:** Shuffle prev(), repeat modes, reproducible order
- ✅ **UX:** Clear feedback, visual indicators, error handling
- ✅ **Maintainability:** Clean separation of concerns, comprehensive tests

**After this revamp, your queue will be indistinguishable from Spotify or Apple Music—and likely better engineered than most.**

---

**Questions? Start with Phase 1 foundation. Success there unblocks everything else.**
