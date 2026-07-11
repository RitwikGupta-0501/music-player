use rusqlite::{Connection, params, OptionalExtension};
use crate::queue::{QueueState, QueueTrack, RepeatMode, QueueMode, ShuffleState};

/// Save queue state to database
pub fn save_queue_state(conn: &Connection, queue: &QueueState) -> Result<i64, String> {
    // Insert queue state
    conn.execute(
        "INSERT INTO queue_state (current_position, repeat_mode, queue_mode, updated_at)
         VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
        params![
            queue.current_position as i32,
            format!("{:?}", queue.repeat_mode),
            format!("{:?}", queue.mode),
        ],
    )
    .map_err(|e| e.to_string())?;

    let queue_state_id = conn.last_insert_rowid();

    // Insert queued tracks
    for (i, track) in queue.tracks.iter().enumerate() {
        conn.execute(
            "INSERT INTO queued_tracks (queue_state_id, instance_id, track_id, position)
             VALUES (?, ?, ?, ?)",
            params![
                queue_state_id,
                &track.instance_id,
                track.track_id,
                i as i32,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    // Insert shuffle order if present
    if let Some(shuffle) = &queue.shuffle_state {
        for (i, instance_id) in shuffle.order.iter().enumerate() {
            conn.execute(
                "INSERT INTO shuffle_order (queue_state_id, instance_id, position, seed)
                 VALUES (?, ?, ?, ?)",
                params![
                    queue_state_id,
                    instance_id,
                    i as i32,
                    shuffle.seed as i32,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(queue_state_id)
}

/// Load queue state from database (returns the most recent)
pub fn load_queue_state(conn: &Connection) -> Result<Option<QueueState>, String> {
    let queue_row = conn
        .query_row(
            "SELECT id, current_position, repeat_mode, queue_mode FROM queue_state
             ORDER BY id DESC LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i32>(1)? as usize,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    match queue_row {
        Some((queue_state_id, current_position, repeat_mode_str, queue_mode_str)) => {
            // Parse repeat mode
            let repeat_mode = match repeat_mode_str.as_str() {
                "Off" => RepeatMode::Off,
                "All" => RepeatMode::All,
                "One" => RepeatMode::One,
                _ => RepeatMode::Off,
            };

            // Parse queue mode
            let queue_mode = match queue_mode_str.as_str() {
                "Shuffle" => QueueMode::Shuffle,
                _ => QueueMode::Normal,
            };

            // Load tracks
            let mut stmt = conn
                .prepare(
                    "SELECT instance_id, track_id, position FROM queued_tracks
                     WHERE queue_state_id = ? ORDER BY position",
                )
                .map_err(|e| e.to_string())?;

            let tracks = stmt
                .query_map(params![queue_state_id], |row| {
                    Ok(QueueTrack {
                        instance_id: row.get(0)?,
                        track_id: row.get(1)?,
                        title: String::new(),  // Will be fetched separately
                        artist: None,
                        file_path: String::new(),
                        album_id: None,
                        track_number: None,
                    })
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            // Hydrate tracks with data from tracks table
            let hydrated_tracks = hydrate_tracks(conn, tracks)?;

            // Load shuffle order if present
            let shuffle_state = load_shuffle_state(conn, queue_state_id)?;

            Ok(Some(QueueState {
                tracks: hydrated_tracks,
                current_position,
                repeat_mode,
                mode: queue_mode,
                shuffle_state,
            }))
        }
        None => Ok(None),
    }
}

/// Load shuffle state for a queue
fn load_shuffle_state(conn: &Connection, queue_state_id: i64) -> Result<Option<ShuffleState>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT instance_id, position, seed FROM shuffle_order
             WHERE queue_state_id = ? ORDER BY position LIMIT 1",
        )
        .map_err(|e| e.to_string())?;

    let has_shuffle = stmt
        .exists(params![queue_state_id])
        .map_err(|e| e.to_string())?;

    if !has_shuffle {
        return Ok(None);
    }

    // Fetch all shuffle order entries
    let mut stmt = conn
        .prepare(
            "SELECT instance_id, position, seed FROM shuffle_order
             WHERE queue_state_id = ? ORDER BY position",
        )
        .map_err(|e| e.to_string())?;

    let order: Vec<String> = stmt
        .query_map(params![queue_state_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let seed = stmt
        .query_row(params![queue_state_id], |row| row.get::<_, i32>(2))
        .map_err(|e| e.to_string())? as u64;

    Ok(Some(ShuffleState {
        order,
        cursor: 0,
        seed,
        regenerate_on_repeat: true,
    }))
}

/// Hydrate minimal tracks with full data from tracks table
fn hydrate_tracks(conn: &Connection, tracks: Vec<QueueTrack>) -> Result<Vec<QueueTrack>, String> {
    let mut hydrated = Vec::new();

    for track in tracks {
        let full_track = conn
            .query_row(
                "SELECT title, artist, album_id, track_number, file_path FROM tracks WHERE id = ?",
                params![track.track_id],
                |row| {
                    Ok(QueueTrack {
                        instance_id: track.instance_id.clone(),
                        track_id: track.track_id,
                        title: row.get(0)?,
                        artist: row.get(1)?,
                        album_id: row.get(2)?,
                        track_number: row.get(3)?,
                        file_path: row.get(4)?,
                    })
                },
            )
            .map_err(|e| format!("Failed to hydrate track {}: {}", track.track_id, e))?;

        hydrated.push(full_track);
    }

    Ok(hydrated)
}

/// Log queue action for analytics
pub fn log_action(
    conn: &Connection,
    action: &str,
    from_pos: Option<usize>,
    to_pos: Option<usize>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO queue_history (action, from_position, to_position, created_at)
         VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
        params![
            action,
            from_pos.map(|p| p as i32),
            to_pos.map(|p| p as i32),
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Create a queue snapshot (for undo/redo in future)
pub fn create_snapshot(
    conn: &Connection,
    queue_state_id: i64,
    description: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO queue_snapshots (queue_state_id, description, created_at)
         VALUES (?, ?, CURRENT_TIMESTAMP)",
        params![queue_state_id, description],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Clear old queue states (keep only last 5 for recovery)
pub fn cleanup_old_states(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "DELETE FROM queue_state WHERE id NOT IN (
            SELECT id FROM queue_state ORDER BY id DESC LIMIT 5
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_queue_persistence_roundtrip() {
        // This would require a test database setup
        // Deferred to integration tests
    }
}
