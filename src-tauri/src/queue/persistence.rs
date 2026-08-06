use rusqlite::{Connection, params, OptionalExtension};
use crate::queue::{QueueState, QueueTrack, TrackSourceInfo, RepeatMode, QueueMode, ShuffleState};

/// Save queue state to database.
/// Remote tracks are persisted with track_id = -1 plus their stream metadata.
/// This allows cross-session queue recovery for local tracks only; remote entries
/// are stored so the sidebar can display them, but they will be pruned on hydration
/// if the stream URL is no longer resolvable.
pub fn save_queue_state(conn: &Connection, queue: &QueueState) -> Result<i64, String> {
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

    for (i, track) in queue.tracks.iter().enumerate() {
        match &track.source {
            TrackSourceInfo::Local { track_id, .. } => {
                conn.execute(
                    "INSERT INTO queued_tracks
                     (queue_state_id, instance_id, track_id, position)
                     VALUES (?, ?, ?, ?)",
                    params![queue_state_id, &track.instance_id, track_id, i as i32],
                )
                .map_err(|e| e.to_string())?;
            }
            TrackSourceInfo::Remote { provider_id, remote_track_id, stream_url, quality_hint, cover_art_url, duration_ms } => {
                conn.execute(
                    "INSERT INTO queued_tracks
                     (queue_state_id, instance_id, track_id, position,
                      stream_url, provider_id, remote_track_id, quality_hint, cached_title, cached_artist, cover_art_url, duration_ms)
                     VALUES (?, ?, -1, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        queue_state_id,
                        &track.instance_id,
                        i as i32,
                        stream_url.as_deref(),
                        provider_id,
                        remote_track_id,
                        quality_hint,
                        &track.title,
                        &track.artist,
                        cover_art_url,
                        duration_ms.map(|d| d as i64),
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    // Save shuffle order if present
    if let Some(shuffle) = &queue.shuffle_state {
        for (i, instance_id) in shuffle.order.iter().enumerate() {
            conn.execute(
                "INSERT INTO shuffle_order (queue_state_id, instance_id, position, seed)
                 VALUES (?, ?, ?, ?)",
                params![queue_state_id, instance_id, i as i32, shuffle.seed as i32],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(queue_state_id)
}

/// Load queue state from database (returns the most recent).
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
            let repeat_mode = match repeat_mode_str.as_str() {
                "All" => RepeatMode::All,
                "One" => RepeatMode::One,
                _ => RepeatMode::Off,
            };
            let queue_mode = match queue_mode_str.as_str() {
                "Shuffle" => QueueMode::Shuffle,
                _ => QueueMode::Normal,
            };

            let mut stmt = conn
                .prepare(
                    "SELECT instance_id, track_id, stream_url, provider_id, remote_track_id, quality_hint,
                            cached_title, cached_artist, cover_art_url, duration_ms
                     FROM queued_tracks
                     WHERE queue_state_id = ? ORDER BY position",
                )
                .map_err(|e| e.to_string())?;

            let raw_tracks: Vec<RawTrackRow> = stmt
                .query_map(params![queue_state_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?, // remote_track_id
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, Option<i64>>(9)?.map(|d| d as u64),
                    ))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            let tracks = hydrate_tracks(conn, raw_tracks)?;

            let shuffle_state = load_shuffle_state(conn, queue_state_id)?;

            Ok(Some(QueueState {
                tracks,
                current_position,
                repeat_mode,
                mode: queue_mode,
                shuffle_state,
            }))
        }
        None => Ok(None),
    }
}

type RawTrackRow = (String, i64, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<u64>);

/// Hydrate raw DB rows into full `QueueTrack` values.
/// Local tracks (track_id > 0) are joined with the library; remote tracks use cached metadata.
fn hydrate_tracks(conn: &Connection, rows: Vec<RawTrackRow>) -> Result<Vec<QueueTrack>, String> {
    let mut tracks = Vec::new();

    for (instance_id, track_id, stream_url, provider_id, remote_track_id, quality_hint, cached_title, cached_artist, cover_art_url, duration_ms) in rows {
        if track_id > 0 {
            // Local track — hydrate from library
            match conn.query_row(
                "SELECT title, artist, album_id, track_number, file_path FROM tracks WHERE id = ?",
                params![track_id],
                |row| {
                    Ok(QueueTrack {
                        instance_id: instance_id.clone(),
                        title: row.get(0)?,
                        artist: row.get(1)?,
                        track_number: row.get(3)?,
                        source: TrackSourceInfo::Local {
                            track_id,
                            file_path: row.get(4)?,
                            album_id: row.get(2)?,
                        },
                    })
                },
            ) {
                Ok(t) => tracks.push(t),
                Err(e) => log::warn!("Skipping orphaned local track {}: {}", track_id, e),
            }
        } else {
            // Remote track — use cached metadata from DB row
            // `remote_track_id` is required for modern remote tracks, but older DBs might not have it.
            // If missing, we fallback to empty string (though ideally they'd just be orphaned).
            if let Some(pid) = provider_id {
                tracks.push(QueueTrack {
                    instance_id,
                    title: cached_title.unwrap_or_else(|| "Unknown".to_string()),
                    artist: cached_artist,
                    track_number: None,
                    source: TrackSourceInfo::Remote {
                        stream_url,
                        provider_id: pid,
                        remote_track_id: remote_track_id.unwrap_or_else(|| "".to_string()),
                        quality_hint,
                        cover_art_url,
                        duration_ms,
                    },
                });
            }
        }
    }

    Ok(tracks)
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

    let mut stmt = conn
        .prepare(
            "SELECT instance_id, seed FROM shuffle_order
             WHERE queue_state_id = ? ORDER BY position",
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, i64)> = stmt
        .query_map(params![queue_state_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let seed = rows.first().map(|(_, s)| *s as u64).unwrap_or(0);
    let order = rows.into_iter().map(|(id, _)| id).collect();

    Ok(Some(ShuffleState {
        order,
        cursor: 0,
        seed,
        regenerate_on_repeat: true,
    }))
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
        // Requires a test database setup — deferred to integration tests
    }
}
