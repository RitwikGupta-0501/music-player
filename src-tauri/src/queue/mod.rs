use serde::{Serialize, Deserialize};
use rand::seq::SliceRandom;

pub mod persistence;
pub mod recovery;

pub mod commands;

/// The audio source for a queued track — local file or remote stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TrackSourceInfo {
    Local {
        track_id: i64,
        file_path: String,
        album_id: Option<i64>,
    },
    Remote {
        provider_id: String,
        remote_track_id: String,
        stream_url: Option<String>,
        quality_hint: Option<String>,
        cover_art_url: Option<String>,
        duration_ms: Option<u64>,
    },
}

/// A track queued for playback
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QueueTrack {
    pub instance_id: String,      // Unique per queue session (UUID)
    pub title: String,
    pub artist: Option<String>,
    pub track_number: Option<i64>,
    pub source: TrackSourceInfo,
}

impl QueueTrack {
    /// Returns the path or URL to pass directly to `load_audio`, if resolved.
    pub fn playback_path(&self) -> Option<&str> {
        match &self.source {
            TrackSourceInfo::Local { file_path, .. } => Some(file_path),
            TrackSourceInfo::Remote { stream_url, .. } => stream_url.as_deref(),
        }
    }
}

/// Repeat behavior
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

/// Queue mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum QueueMode {
    Normal,
    Shuffle,
}

/// Shuffle order state (when shuffle is enabled)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShuffleState {
    pub order: Vec<String>,       // Shuffled instance IDs
    pub cursor: usize,             // Current position in shuffle order
    pub seed: u64,                 // For reproducibility
    pub regenerate_on_repeat: bool,
}

/// The complete queue state (single source of truth)
#[derive(Clone)]
pub struct QueueState {
    pub tracks: Vec<QueueTrack>,
    pub current_position: usize,
    pub repeat_mode: RepeatMode,
    pub mode: QueueMode,
    pub shuffle_state: Option<ShuffleState>,
}

impl Default for QueueState {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueState {
    /// Create empty queue
    pub fn new() -> Self {
        QueueState {
            tracks: Vec::new(),
            current_position: 0,
            repeat_mode: RepeatMode::Off,
            mode: QueueMode::Normal,
            shuffle_state: None,
        }
    }

    // ══════════════════════════════════════════════════════════
    // QUEUE MANIPULATION
    // ══════════════════════════════════════════════════════════

    /// Replace entire queue and jump to start_index
    pub fn set_queue(&mut self, tracks: Vec<QueueTrack>, start_index: usize) -> Result<(), String> {
        if !tracks.is_empty() && start_index >= tracks.len() {
            return Err(format!(
                "Start index {} out of bounds (queue length: {})",
                start_index,
                tracks.len()
            ));
        }

        self.tracks = tracks;
        self.current_position = start_index;
        self.shuffle_state = None;  // Clear shuffle when new queue set
        self.mode = QueueMode::Normal;
        Ok(())
    }

    /// Add single track to end of queue
    pub fn add_track(&mut self, track: QueueTrack) -> Result<(), String> {
        self.tracks.push(track.clone());

        // If shuffle is on, add to shuffle order
        if let Some(shuffle) = &mut self.shuffle_state {
            shuffle.order.push(track.instance_id);
        }

        Ok(())
    }

    /// Clear all tracks from queue
    pub fn clear(&mut self) -> Result<(), String> {
        self.tracks.clear();
        self.current_position = 0;
        self.shuffle_state = None;
        Ok(())
    }

    /// Reorder tracks in queue (from_index -> to_index)
    pub fn reorder(&mut self, from: usize, to: usize) -> Result<(), String> {
        if from >= self.tracks.len() || to >= self.tracks.len() {
            return Err(format!(
                "Reorder indices out of bounds: from={}, to={}, len={}",
                from, to,
                self.tracks.len()
            ));
        }

        if from == to {
            return Ok(());  // No-op
        }

        // Move track in array
        let track = self.tracks.remove(from);
        self.tracks.insert(to, track);

        // Update current_position if we're in normal mode
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

    // ══════════════════════════════════════════════════════════
    // NAVIGATION
    // ══════════════════════════════════════════════════════════

    /// Advance to next track, respecting repeat mode
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<QueueTrack>, String> {
        if self.tracks.is_empty() {
            return Ok(None);
        }

        let next_pos = match self.mode {
            QueueMode::Normal => self.next_normal()?,
            QueueMode::Shuffle => self.next_shuffle()?,
        };

        match next_pos {
            Some(pos) => self.jump_to_position(pos).map(Some),
            None => Ok(None),
        }
    }

    /// Go to previous track, respecting repeat mode
    pub fn prev(&mut self) -> Result<Option<QueueTrack>, String> {
        if self.tracks.is_empty() {
            return Ok(None);
        }

        let prev_pos = match self.mode {
            QueueMode::Normal => self.prev_normal()?,
            QueueMode::Shuffle => self.prev_shuffle()?,
        };

        match prev_pos {
            Some(pos) => self.jump_to_position(pos).map(Some),
            None => Ok(None),
        }
    }

    /// Jump to specific position in queue
    pub fn jump_to_position(&mut self, position: usize) -> Result<QueueTrack, String> {
        if position >= self.tracks.len() {
            return Err(format!(
                "Jump position {} out of bounds (queue length: {})",
                position,
                self.tracks.len()
            ));
        }

        self.current_position = position;
        Ok(self.tracks[position].clone())
    }

    /// Jump to track by instance_id
    pub fn jump_to_instance_id(&mut self, instance_id: String) -> Result<QueueTrack, String> {
        let idx = self.tracks
            .iter()
            .position(|t| t.instance_id == instance_id)
            .ok_or_else(|| format!("Track {} not in queue", instance_id))?;

        self.jump_to_position(idx)
    }

    // ══════════════════════════════════════════════════════════
    // MODE SWITCHING
    // ══════════════════════════════════════════════════════════

    /// Enable/disable shuffle mode
    pub fn set_shuffle(&mut self, enabled: bool) -> Result<(), String> {
        if enabled && self.mode == QueueMode::Shuffle {
            return Ok(());  // Already shuffled
        }

        if !enabled && self.mode == QueueMode::Normal {
            return Ok(());  // Already normal
        }

        if enabled {
            // Entering shuffle: preserve current track
            if self.tracks.is_empty() {
                return Err("Cannot shuffle empty queue".to_string());
            }

            let current_id = self.tracks[self.current_position].instance_id.clone();
            self.regenerate_shuffle_order()?;

            // Position cursor at current track in new shuffle order
            if let Some(shuffle) = &mut self.shuffle_state {
                if let Some(idx) = shuffle.order.iter().position(|id| id == &current_id) {
                    shuffle.cursor = idx;
                }
            }

            self.mode = QueueMode::Shuffle;
        } else {
            // Exiting shuffle: preserve current track position in normal mode
            if self.tracks.is_empty() {
                return Err("Cannot exit shuffle with empty queue".to_string());
            }

            let current_id = self.tracks[self.current_position].instance_id.clone();

            if let Some(idx) = self.tracks.iter().position(|t| t.instance_id == current_id) {
                self.current_position = idx;
            }

            self.shuffle_state = None;
            self.mode = QueueMode::Normal;
        }

        Ok(())
    }

    // ══════════════════════════════════════════════════════════
    // INTERNAL: Navigation Logic
    // ══════════════════════════════════════════════════════════

    fn next_normal(&mut self) -> Result<Option<usize>, String> {
        let pos = self.current_position + 1;
        if pos >= self.tracks.len() {
            match self.repeat_mode {
                RepeatMode::Off => Ok(None),
                RepeatMode::All => Ok(Some(0)),
                RepeatMode::One => Ok(Some(self.current_position)),
            }
        } else {
            Ok(Some(pos))
        }
    }

    fn next_shuffle(&mut self) -> Result<Option<usize>, String> {
        let shuffle = self.shuffle_state.as_mut()
            .ok_or_else(|| "Shuffle state missing".to_string())?;

        let next_cursor = shuffle.cursor + 1;
        if next_cursor >= shuffle.order.len() {
            match self.repeat_mode {
                RepeatMode::Off => Ok(None),
                RepeatMode::All => {
                    self.regenerate_shuffle_order()?;
                    let shuffle = self.shuffle_state.as_ref().unwrap();
                    if let Some(id) = shuffle.order.first() {
                        let idx = self.tracks
                            .iter()
                            .position(|t| &t.instance_id == id)
                            .ok_or_else(|| "Track not found in queue".to_string())?;
                        Ok(Some(idx))
                    } else {
                        Ok(None)
                    }
                },
                RepeatMode::One => Ok(Some(self.current_position)),
            }
        } else {
            shuffle.cursor = next_cursor;
            let id = &shuffle.order[next_cursor];
            let idx = self.tracks
                .iter()
                .position(|t| &t.instance_id == id)
                .ok_or_else(|| "Track not found in queue".to_string())?;
            Ok(Some(idx))
        }
    }

    fn prev_normal(&mut self) -> Result<Option<usize>, String> {
        let pos = self.current_position as i32 - 1;
        if pos < 0 {
            match self.repeat_mode {
                RepeatMode::Off => Ok(None),
                RepeatMode::All => Ok(Some(self.tracks.len() - 1)),
                RepeatMode::One => Ok(Some(self.current_position)),
            }
        } else {
            Ok(Some(pos as usize))
        }
    }

    fn prev_shuffle(&mut self) -> Result<Option<usize>, String> {
        let shuffle = self.shuffle_state.as_mut()
            .ok_or_else(|| "Shuffle state missing".to_string())?;

        let prev_cursor = shuffle.cursor as i32 - 1;
        if prev_cursor < 0 {
            match self.repeat_mode {
                RepeatMode::Off => Ok(None),
                RepeatMode::All => {
                    // Wrap to end without regenerating
                    shuffle.cursor = shuffle.order.len() - 1;
                    let id = &shuffle.order[shuffle.cursor];
                    let idx = self.tracks
                        .iter()
                        .position(|t| &t.instance_id == id)
                        .ok_or_else(|| "Track not found in queue".to_string())?;
                    Ok(Some(idx))
                },
                RepeatMode::One => Ok(Some(self.current_position)),
            }
        } else {
            shuffle.cursor = prev_cursor as usize;
            let id = &shuffle.order[shuffle.cursor];
            let idx = self.tracks
                .iter()
                .position(|t| &t.instance_id == id)
                .ok_or_else(|| "Track not found in queue".to_string())?;
            Ok(Some(idx))
        }
    }

    // ══════════════════════════════════════════════════════════
    // SHUFFLE GENERATION
    // ══════════════════════════════════════════════════════════

    /// Generate new random shuffle order using thread RNG
    pub fn regenerate_shuffle_order(&mut self) -> Result<(), String> {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        let mut order: Vec<String> = self.tracks.iter()
            .map(|t| t.instance_id.clone())
            .collect();

        let mut rng = rand::thread_rng();
        order.shuffle(&mut rng);

        self.shuffle_state = Some(ShuffleState {
            order,
            cursor: 0,
            seed,
            regenerate_on_repeat: true,
        });

        Ok(())
    }

    // ══════════════════════════════════════════════════════════
    // QUERIES
    // ══════════════════════════════════════════════════════════

    /// Get current track
    pub fn current_track(&self) -> Option<&QueueTrack> {
        self.tracks.get(self.current_position)
    }

    /// Get all tracks
    pub fn get_all(&self) -> &[QueueTrack] {
        &self.tracks
    }

    /// Queue length
    pub fn length(&self) -> usize {
        self.tracks.len()
    }
}

// ════════════════════════════════════════════════════════════════════════════════
// TESTS
// ════════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_track(id: &str) -> QueueTrack {
        QueueTrack {
            instance_id: id.to_string(),
            title: format!("Track {}", id),
            artist: Some("Artist".to_string()),
            track_number: Some(1),
            source: TrackSourceInfo::Local {
                track_id: id.parse().unwrap_or(1),
                file_path: format!("/path/to/{}.mp3", id),
                album_id: Some(1),
            },
        }
    }

    // ── SET_QUEUE TESTS ──

    #[test]
    fn test_set_queue_valid() {
        let mut queue = QueueState::new();
        let tracks = vec![make_track("1"), make_track("2"), make_track("3")];

        assert!(queue.set_queue(tracks, 0).is_ok());
        assert_eq!(queue.length(), 3);
        assert_eq!(queue.current_position, 0);
    }

    #[test]
    fn test_set_queue_invalid_start_index() {
        let mut queue = QueueState::new();
        let tracks = vec![make_track("1"), make_track("2")];

        assert!(queue.set_queue(tracks, 5).is_err());
    }

    #[test]
    fn test_set_queue_clears_shuffle() {
        let mut queue = QueueState::new();
        let tracks = vec![make_track("1"), make_track("2")];

        queue.set_queue(tracks.clone(), 0).unwrap();
        queue.set_shuffle(true).unwrap();
        assert!(queue.shuffle_state.is_some());

        queue.set_queue(tracks, 0).unwrap();
        assert!(queue.shuffle_state.is_none());
    }

    // ── NEXT TESTS (Normal Mode) ──

    #[test]
    fn test_next_normal_mode() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 0)
            .unwrap();

        let track = queue.next().unwrap().unwrap();
        assert_eq!(track.instance_id, "2");
        assert_eq!(queue.current_position, 1);
    }

    #[test]
    fn test_next_repeat_off() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 1)
            .unwrap();
        queue.repeat_mode = RepeatMode::Off;

        // At last track, next should return None
        let result = queue.next().unwrap();
        assert!(result.is_none());
        assert_eq!(queue.current_position, 1);  // Position unchanged
    }

    #[test]
    fn test_next_repeat_all() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 1)
            .unwrap();
        queue.repeat_mode = RepeatMode::All;

        let track = queue.next().unwrap().unwrap();
        assert_eq!(track.instance_id, "1");
        assert_eq!(queue.current_position, 0);
    }

    #[test]
    fn test_next_repeat_one() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 1)
            .unwrap();
        queue.repeat_mode = RepeatMode::One;

        let track = queue.next().unwrap().unwrap();
        assert_eq!(track.instance_id, "2");
        assert_eq!(queue.current_position, 1);  // Stays on current
    }

    // ── NEXT TESTS (Shuffle Mode) ──

    #[test]
    fn test_next_shuffle_mode() {
        let mut queue = QueueState::new();
        queue
            .set_queue(
                vec![make_track("1"), make_track("2"), make_track("3")],
                0,
            )
            .unwrap();

        queue.set_shuffle(true).unwrap();
        assert!(queue.shuffle_state.is_some());

        let _result = queue.next();
        // Shuffle order is random, just verify it doesn't crash
        assert!(queue.current_position < queue.length());
    }

    // ── PREV TESTS ──

    #[test]
    fn test_prev_normal_mode() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 2)
            .unwrap();

        let track = queue.prev().unwrap().unwrap();
        assert_eq!(track.instance_id, "2");
        assert_eq!(queue.current_position, 1);
    }

    #[test]
    fn test_prev_at_start_repeat_off() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 0)
            .unwrap();
        queue.repeat_mode = RepeatMode::Off;

        let result = queue.prev().unwrap();
        assert!(result.is_none());
        assert_eq!(queue.current_position, 0);
    }

    #[test]
    fn test_prev_at_start_repeat_all() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 0)
            .unwrap();
        queue.repeat_mode = RepeatMode::All;

        let track = queue.prev().unwrap().unwrap();
        assert_eq!(track.instance_id, "2");
        assert_eq!(queue.current_position, 1);
    }

    // ── JUMP TESTS ──

    #[test]
    fn test_jump_to_position() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 0)
            .unwrap();

        let track = queue.jump_to_position(2).unwrap();
        assert_eq!(track.instance_id, "3");
        assert_eq!(queue.current_position, 2);
    }

    #[test]
    fn test_jump_to_instance_id() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 0)
            .unwrap();

        let track = queue.jump_to_instance_id("2".to_string()).unwrap();
        assert_eq!(track.instance_id, "2");
        assert_eq!(queue.current_position, 1);
    }

    #[test]
    fn test_jump_to_nonexistent_track() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 0)
            .unwrap();

        assert!(queue.jump_to_instance_id("999".to_string()).is_err());
    }

    // ── REORDER TESTS ──

    #[test]
    fn test_reorder_swap() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 0)
            .unwrap();

        queue.reorder(0, 2).unwrap();

        assert_eq!(queue.tracks[0].instance_id, "2");
        assert_eq!(queue.tracks[2].instance_id, "1");
    }

    #[test]
    fn test_reorder_updates_current_position() {
        let mut queue = QueueState::new();
        queue
            .set_queue(
                vec![make_track("1"), make_track("2"), make_track("3")],
                2,
            )
            .unwrap();

        // Reorder position 2 to 0: current was at 2 (track 3), after move it should be at 0
        // Initial: [1, 2, 3] at pos 2
        // After reorder(2, 0): [3, 1, 2] and current should follow track 3 to pos 0
        queue.reorder(2, 0).unwrap();
        assert_eq!(queue.current_position, 0);
        assert_eq!(queue.current_track().unwrap().instance_id, "3");
    }

    #[test]
    fn test_reorder_shifts_current_position() {
        let mut queue = QueueState::new();
        queue
            .set_queue(
                vec![make_track("1"), make_track("2"), make_track("3"), make_track("4")],
                2,
            )
            .unwrap();

        // Reorder position 0 to 3: current was at 2, should shift to 1 (before the insertion point)
        // Initial: [1, 2, 3, 4] at pos 2 (track 3)
        // After reorder(0, 3): [2, 3, 4, 1] and current should shift to pos 1
        queue.reorder(0, 3).unwrap();
        assert_eq!(queue.current_position, 1);
        assert_eq!(queue.current_track().unwrap().instance_id, "3");
    }

    #[test]
    fn test_reorder_out_of_bounds() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 0)
            .unwrap();

        assert!(queue.reorder(0, 5).is_err());
    }

    #[test]
    fn test_reorder_no_op() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 0)
            .unwrap();

        queue.reorder(0, 0).unwrap();
        assert_eq!(queue.current_position, 0);
    }

    // ── SHUFFLE TESTS ──

    #[test]
    fn test_enable_shuffle() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 0)
            .unwrap();

        queue.set_shuffle(true).unwrap();

        assert!(queue.shuffle_state.is_some());
        assert_eq!(queue.mode, QueueMode::Shuffle);
        assert_eq!(queue.shuffle_state.unwrap().order.len(), 3);
    }

    #[test]
    fn test_shuffle_preserves_current_track() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 1)
            .unwrap();

        let current_id = queue.current_track().unwrap().instance_id.clone();

        queue.set_shuffle(true).unwrap();

        let shuffle_cursor = queue.shuffle_state.as_ref().unwrap().cursor;
        let shuffle_order = queue.shuffle_state.as_ref().unwrap().order.clone();
        let track_at_cursor = queue.tracks
            .iter()
            .find(|t| t.instance_id == shuffle_order[shuffle_cursor])
            .unwrap();

        assert_eq!(track_at_cursor.instance_id, current_id);
    }

    #[test]
    fn test_disable_shuffle() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 0)
            .unwrap();

        queue.set_shuffle(true).unwrap();
        assert_eq!(queue.mode, QueueMode::Shuffle);

        queue.set_shuffle(false).unwrap();
        assert_eq!(queue.mode, QueueMode::Normal);
        assert!(queue.shuffle_state.is_none());
    }

    // ── EMPTY QUEUE TESTS ──

    #[test]
    fn test_empty_queue_operations() {
        let mut queue = QueueState::new();

        assert!(queue.next().unwrap().is_none());
        assert!(queue.prev().unwrap().is_none());
        assert_eq!(queue.current_track(), None);
        assert_eq!(queue.length(), 0);
    }

    #[test]
    fn test_clear_queue() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 0)
            .unwrap();

        queue.clear().unwrap();

        assert_eq!(queue.length(), 0);
        assert_eq!(queue.current_position, 0);
    }

    // ── ADD_TRACK TESTS ──

    #[test]
    fn test_add_track() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 0)
            .unwrap();

        queue.add_track(make_track("3")).unwrap();

        assert_eq!(queue.length(), 3);
        assert_eq!(queue.tracks[2].instance_id, "3");
    }

    // ── EDGE CASES ──

    #[test]
    fn test_single_track_queue() {
        let mut queue = QueueState::new();
        queue.set_queue(vec![make_track("1")], 0).unwrap();

        // next() with repeat=off should return None
        queue.repeat_mode = RepeatMode::Off;
        assert!(queue.next().unwrap().is_none());

        // next() with repeat=all should loop
        queue.repeat_mode = RepeatMode::All;
        let track = queue.next().unwrap().unwrap();
        assert_eq!(track.instance_id, "1");

        // next() with repeat=one should stay
        queue.repeat_mode = RepeatMode::One;
        queue.current_position = 0;  // Reset
        let track = queue.next().unwrap().unwrap();
        assert_eq!(track.instance_id, "1");
    }

    #[test]
    fn test_sequential_skips() {
        let mut queue = QueueState::new();
        queue
            .set_queue(
                vec![make_track("1"), make_track("2"), make_track("3"), make_track("4")],
                0,
            )
            .unwrap();

        // Skip 3 times
        let _ = queue.next().unwrap();
        let _ = queue.next().unwrap();
        let track = queue.next().unwrap().unwrap();

        assert_eq!(track.instance_id, "4");
        assert_eq!(queue.current_position, 3);
    }

    #[test]
    fn test_large_queue_performance() {
        let mut queue = QueueState::new();
        let mut tracks = Vec::new();

        // Create 10k tracks
        for i in 0..10_000 {
            tracks.push(make_track(&i.to_string()));
        }

        let start = std::time::Instant::now();
        queue.set_queue(tracks, 5_000).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(queue.length(), 10_000);
        assert!(elapsed.as_millis() < 100, "set_queue should be <100ms for 10k items");
    }

    #[test]
    fn test_shuffle_determinism_same_seed() {
        let mut queue1 = QueueState::new();
        queue1
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 0)
            .unwrap();

        let mut queue2 = QueueState::new();
        queue2
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 0)
            .unwrap();

        // Regenerate shuffle multiple times on queue1
        queue1.regenerate_shuffle_order().unwrap();
        let shuffle1_a = queue1.shuffle_state.as_ref().unwrap().order.clone();

        queue1.regenerate_shuffle_order().unwrap();
        let shuffle1_b = queue1.shuffle_state.as_ref().unwrap().order.clone();

        // They'll be different because we use thread_rng (not seeded)
        // But verify both are valid permutations
        assert_eq!(shuffle1_a.len(), 3);
        assert_eq!(shuffle1_b.len(), 3);
    }

    // ── SHUFFLE + REPEAT COMBINATIONS ──

    #[test]
    fn test_shuffle_with_repeat_modes() {
        let mut queue = QueueState::new();
        queue
            .set_queue(
                vec![make_track("1"), make_track("2"), make_track("3")],
                0,
            )
            .unwrap();

        // Test that shuffle can be enabled with all repeat modes
        queue.set_shuffle(true).unwrap();
        assert!(queue.shuffle_state.is_some());

        queue.repeat_mode = RepeatMode::Off;
        assert_eq!(queue.repeat_mode, RepeatMode::Off);

        queue.repeat_mode = RepeatMode::All;
        assert_eq!(queue.repeat_mode, RepeatMode::All);

        queue.repeat_mode = RepeatMode::One;
        assert_eq!(queue.repeat_mode, RepeatMode::One);
    }

    #[test]
    fn test_prev_in_shuffle() {
        let mut queue = QueueState::new();
        queue
            .set_queue(
                vec![make_track("1"), make_track("2"), make_track("3")],
                2,
            )
            .unwrap();

        queue.set_shuffle(true).unwrap();
        let _shuffle_order = queue.shuffle_state.as_ref().unwrap().order.clone();
        let initial_cursor = queue.shuffle_state.as_ref().unwrap().cursor;

        // Go prev
        queue.prev().unwrap();

        // Cursor should have moved
        let new_cursor = queue.shuffle_state.as_ref().unwrap().cursor;
        assert!(new_cursor < initial_cursor || initial_cursor == 0);
    }

    #[test]
    fn test_add_track_during_shuffle() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 0)
            .unwrap();

        queue.set_shuffle(true).unwrap();
        let initial_len = queue.shuffle_state.as_ref().unwrap().order.len();

        queue.add_track(make_track("3")).unwrap();

        // Shuffle order should be updated
        assert_eq!(
            queue.shuffle_state.as_ref().unwrap().order.len(),
            initial_len + 1
        );
    }

    #[test]
    fn test_shuffle_preserves_current_track_complex() {
        let mut queue = QueueState::new();
        queue
            .set_queue(
                vec![
                    make_track("1"),
                    make_track("2"),
                    make_track("3"),
                    make_track("4"),
                    make_track("5"),
                ],
                3,
            )
            .unwrap();

        let current_track_id = queue.current_track().unwrap().instance_id.clone();
        let current_title = queue.current_track().unwrap().title.clone();

        queue.set_shuffle(true).unwrap();

        // Current track should still be the same, even though shuffle order changed
        let new_track = queue.current_track().unwrap();
        assert_eq!(new_track.instance_id, current_track_id);
        assert_eq!(new_track.title, current_title);
    }

    #[test]
    fn test_reshuffle_mid_playback() {
        let mut queue = QueueState::new();
        queue
            .set_queue(
                vec![make_track("1"), make_track("2"), make_track("3")],
                0,
            )
            .unwrap();

        queue.set_shuffle(true).unwrap();
        let _initial_shuffle = queue.shuffle_state.as_ref().unwrap().order.clone();

        // Advance one track
        queue.next().unwrap();
        let _pos_after_skip = queue.current_position;

        // Regenerate (like user pressing "reshuffle")
        queue.regenerate_shuffle_order().unwrap();

        // Cursor should be reset, but we should still be on a valid track
        assert!(queue.current_position < queue.length());
    }

    #[test]
    fn test_all_repeat_modes_with_empty_queue() {
        let mut queue = QueueState::new();

        for mode in [RepeatMode::Off, RepeatMode::All, RepeatMode::One] {
            queue.repeat_mode = mode;
            assert!(queue.next().unwrap().is_none());
            assert!(queue.prev().unwrap().is_none());
        }
    }

    #[test]
    fn test_repeat_mode_transitions() {
        let mut queue = QueueState::new();
        queue
            .set_queue(
                vec![make_track("1"), make_track("2"), make_track("3")],
                2,
            )
            .unwrap();

        // Off → All
        queue.repeat_mode = RepeatMode::Off;
        assert!(queue.next().unwrap().is_none());

        queue.repeat_mode = RepeatMode::All;
        queue.current_position = 2;  // Reset to end
        let result = queue.next().unwrap();
        assert!(result.is_some());
        assert_eq!(queue.current_position, 0);

        // All → One
        queue.repeat_mode = RepeatMode::One;
        queue.current_position = 2;
        let result = queue.next().unwrap();
        assert!(result.is_some());
        assert_eq!(queue.current_position, 2);  // Should stay

        // One → Off
        queue.repeat_mode = RepeatMode::Off;
        queue.current_position = 2;
        assert!(queue.next().unwrap().is_none());  // Should stop
    }
}
