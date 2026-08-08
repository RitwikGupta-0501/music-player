use crate::queue::{QueueMode, QueueState, QueueTrack, RepeatMode};
use crate::{feature_flags, telemetry};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

/// Event payload sent to frontend when queue changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueChangeEvent {
    pub tracks: Vec<QueueTrack>,
    pub current_position: usize,
    pub current_track: Option<QueueTrack>,
    pub repeat_mode: RepeatMode,
    pub queue_mode: QueueMode,
}

/// Convert QueueState to event payload
fn queue_to_event(queue: &QueueState) -> QueueChangeEvent {
    QueueChangeEvent {
        tracks: queue.get_all().to_vec(),
        current_position: queue.current_position,
        current_track: queue.current_track().cloned(),
        repeat_mode: queue.repeat_mode,
        queue_mode: queue.mode,
    }
}

// ════════════════════════════════════════════════════════════════════════════════
// QUEUE MANAGEMENT COMMANDS
// ════════════════════════════════════════════════════════════════════════════════

/// Set queue and start playing at index
///
/// # Arguments
/// * `tracks` - Tracks to load into queue
/// * `start_index` - Index to start playing (0-based)
///
/// # Returns
/// Queue state after change, or error string
#[tauri::command]
pub async fn set_queue(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
    tracks: Vec<QueueTrack>,
    start_index: usize,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| {
        let err = e.to_string();
        telemetry::record_error("set_queue", &err);
        err
    })?;

    let track_count = tracks.len();
    queue.set_queue(tracks, start_index).inspect_err(|e| {
        telemetry::record_error("set_queue", e);
    })?;

    log::info!(
        "Queue set with {} tracks, starting at index {}",
        track_count,
        start_index
    );

    let event = queue_to_event(&queue);
    let _ = app_handle.emit("queue-changed", &event);

    Ok(event)
}

/// Add single track to end of queue
#[tauri::command]
pub async fn add_to_queue(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
    track: QueueTrack,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;
    queue.add_track(track)?;

    let event = queue_to_event(&queue);
    let _ = app_handle.emit("queue-changed", &event);

    Ok(event)
}

/// Clear all tracks from queue
#[tauri::command]
pub async fn clear_queue(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<QueueChangeEvent, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _ = state.db_tx.send(crate::db::DbRequest::GetSetting {
        key: "keep_playing_on_queue_clear".to_string(),
        resp: tx,
    });

    let keep_playing = match rx.await {
        Ok(Ok(Some(val))) => val == "true",
        _ => false,
    };

    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;
    queue.clear()?;

    if !keep_playing {
        if let Ok(tx) = state.audio_tx.lock() {
            let _ = tx.send(crate::audio::AudioCommand::Stop);
        }
    }

    let event = queue_to_event(&queue);
    let _ = app_handle.emit("queue-changed", &event);

    Ok(event)
}

// ════════════════════════════════════════════════════════════════════════════════
// NAVIGATION COMMANDS
// ════════════════════════════════════════════════════════════════════════════════

/// Skip forward N tracks (batch operation)
///
/// This is a batch operation: skip multiple times in single IPC call
/// much faster than calling skip_forward(1) N times
#[tauri::command]
pub async fn skip_forward(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
    count: u32,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| {
        let err = e.to_string();
        telemetry::record_error("skip_forward", &err);
        err
    })?;

    for _ in 0..count {
        queue.next().inspect_err(|e| {
            telemetry::record_error("skip_forward", e);
        })?;
    }

    log::debug!("Skipped forward {} track(s)", count);

    let event = queue_to_event(&queue);
    let _ = app_handle.emit("queue-changed", &event);

    Ok(event)
}

/// Skip backward N tracks (batch operation)
#[tauri::command]
pub async fn skip_backward(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
    count: u32,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;

    for _ in 0..count {
        queue.prev()?;
    }

    let event = queue_to_event(&queue);
    let _ = app_handle.emit("queue-changed", &event);

    Ok(event)
}

/// Jump to specific position in queue
#[tauri::command]
pub async fn jump_to_position(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
    position: usize,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;
    queue.jump_to_position(position)?;

    let event = queue_to_event(&queue);
    let _ = app_handle.emit("queue-changed", &event);

    Ok(event)
}

/// Jump to track by instance_id
#[tauri::command]
pub async fn jump_to_track(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
    instance_id: String,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;
    queue.jump_to_instance_id(instance_id)?;

    let event = queue_to_event(&queue);
    let _ = app_handle.emit("queue-changed", &event);

    Ok(event)
}

// ════════════════════════════════════════════════════════════════════════════════
// REORDERING COMMANDS
// ════════════════════════════════════════════════════════════════════════════════

/// Reorder track in queue (drag-and-drop)
///
/// # Arguments
/// * `from_index` - Current position of track
/// * `to_index` - New position for track
#[tauri::command]
pub async fn reorder_queue(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
    from_index: usize,
    to_index: usize,
) -> Result<QueueChangeEvent, String> {
    if !feature_flags::FEATURE_FLAGS.is_enabled(feature_flags::FeatureFlag::ReorderQueue) {
        return Err("Queue reordering is not enabled".to_string());
    }

    let mut queue = state.queue.lock().map_err(|e| {
        let err = e.to_string();
        telemetry::record_error("reorder_queue", &err);
        err
    })?;

    queue.reorder(from_index, to_index).inspect_err(|e| {
        telemetry::record_error("reorder_queue", e);
    })?;

    log::debug!("Queue reordered: {} -> {}", from_index, to_index);

    let event = queue_to_event(&queue);
    let _ = app_handle.emit("queue-changed", &event);

    Ok(event)
}

// ════════════════════════════════════════════════════════════════════════════════
// MODE CONTROL COMMANDS
// ════════════════════════════════════════════════════════════════════════════════

/// Set repeat mode
///
/// # Arguments
/// * `mode` - One of: "Off", "All", "One"
#[tauri::command]
pub async fn set_repeat_mode(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
    mode: String,
) -> Result<QueueChangeEvent, String> {
    let repeat_mode = match mode.as_str() {
        "Off" => RepeatMode::Off,
        "All" => RepeatMode::All,
        "One" => RepeatMode::One,
        _ => return Err(format!("Invalid repeat mode: {}", mode)),
    };

    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;
    queue.repeat_mode = repeat_mode;

    let event = queue_to_event(&queue);
    let _ = app_handle.emit("queue-changed", &event);

    Ok(event)
}

/// Enable/disable shuffle
#[tauri::command]
pub async fn set_shuffle(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
    enabled: bool,
) -> Result<QueueChangeEvent, String> {
    if !feature_flags::FEATURE_FLAGS.is_enabled(feature_flags::FeatureFlag::ShuffleMode) {
        return Err("Shuffle mode is not enabled".to_string());
    }

    let mut queue = state.queue.lock().map_err(|e| {
        let err = e.to_string();
        telemetry::record_error("set_shuffle", &err);
        err
    })?;

    queue.set_shuffle(enabled).inspect_err(|e| {
        telemetry::record_error("set_shuffle", e);
    })?;

    log::info!("Shuffle toggled: {}", if enabled { "ON" } else { "OFF" });

    let event = queue_to_event(&queue);
    let _ = app_handle.emit("queue-changed", &event);

    Ok(event)
}

// ════════════════════════════════════════════════════════════════════════════════
// QUERY COMMANDS
// ════════════════════════════════════════════════════════════════════════════════

/// Get current queue state (for frontend sync)
#[tauri::command]
pub async fn get_queue(state: State<'_, crate::AppState>) -> Result<QueueChangeEvent, String> {
    let queue = state.queue.lock().map_err(|e| e.to_string())?;
    Ok(queue_to_event(&queue))
}

/// Get queue length
#[tauri::command]
pub async fn get_queue_length(state: State<'_, crate::AppState>) -> Result<usize, String> {
    let queue = state.queue.lock().map_err(|e| e.to_string())?;
    Ok(queue.length())
}

/// Get current track
#[tauri::command]
pub async fn get_current_track(
    state: State<'_, crate::AppState>,
) -> Result<Option<QueueTrack>, String> {
    let queue = state.queue.lock().map_err(|e| e.to_string())?;
    Ok(queue.current_track().cloned())
}

// ════════════════════════════════════════════════════════════════════════════════
// ADVANCED OPERATIONS
// ════════════════════════════════════════════════════════════════════════════════

/// Reshuffle queue without changing current track
///
/// Useful for "randomize" button when shuffle is already on
#[tauri::command]
pub async fn reshuffle(
    app_handle: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<QueueChangeEvent, String> {
    let mut queue = state.queue.lock().map_err(|e| e.to_string())?;

    if queue.mode != QueueMode::Shuffle {
        return Err("Shuffle not enabled".to_string());
    }

    let current_id = queue
        .current_track()
        .ok_or_else(|| "No current track".to_string())?
        .instance_id
        .clone();

    queue.regenerate_shuffle_order()?;

    // Maintain current track position in new shuffle
    if let Some(shuffle) = &mut queue.shuffle_state {
        if let Some(idx) = shuffle.order.iter().position(|id| id == &current_id) {
            shuffle.cursor = idx;
        }
    }

    let event = queue_to_event(&queue);
    let _ = app_handle.emit("queue-changed", &event);

    Ok(event)
}

// ════════════════════════════════════════════════════════════════════════════════
// TESTS
// ════════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::TrackSourceInfo;

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

    #[test]
    fn test_queue_to_event() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 0)
            .unwrap();

        let event = queue_to_event(&queue);

        assert_eq!(event.tracks.len(), 2);
        assert_eq!(event.current_position, 0);
        assert_eq!(event.queue_mode, QueueMode::Normal);
        assert_eq!(event.repeat_mode, RepeatMode::Off);
    }

    #[test]
    fn test_event_includes_current_track() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 1)
            .unwrap();

        let event = queue_to_event(&queue);

        assert!(event.current_track.is_some());
        assert_eq!(event.current_track.unwrap().instance_id, "2");
    }

    #[test]
    fn test_event_shuffle_mode() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2"), make_track("3")], 0)
            .unwrap();
        queue.set_shuffle(true).unwrap();

        let event = queue_to_event(&queue);

        assert_eq!(event.queue_mode, QueueMode::Shuffle);
    }

    #[test]
    fn test_event_repeat_mode() {
        let mut queue = QueueState::new();
        queue
            .set_queue(vec![make_track("1"), make_track("2")], 0)
            .unwrap();
        queue.repeat_mode = RepeatMode::All;

        let event = queue_to_event(&queue);

        assert_eq!(event.repeat_mode, RepeatMode::All);
    }
}
