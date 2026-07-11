use rusqlite::Connection;
use crate::queue::QueueState;
use super::persistence;

/// Attempt to recover queue state on app startup
pub fn recover_on_startup(conn: &Connection) -> Result<QueueState, String> {
    match persistence::load_queue_state(conn) {
        Ok(Some(queue)) => {
            log::info!(
                "Queue recovered: {} tracks, position {}/{}",
                queue.length(),
                queue.current_position,
                queue.length()
            );
            Ok(queue)
        }
        Ok(None) => {
            log::debug!("No previous queue state found, starting fresh");
            Ok(QueueState::new())
        }
        Err(e) => {
            log::warn!("Failed to recover queue: {}, starting fresh", e);
            Ok(QueueState::new())  // Graceful fallback
        }
    }
}

/// Periodically save queue state (called by background task)
pub fn periodic_save(conn: &Connection, queue: &QueueState) -> Result<(), String> {
    // Only save if there's content worth saving
    if queue.length() > 0 {
        persistence::save_queue_state(conn, queue)?;

        // Cleanup old states periodically
        if queue.current_position.is_multiple_of(10) {
            let _ = persistence::cleanup_old_states(conn);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_recovery_graceful_fallback() {
        // Test that recovery fails gracefully on empty database
        // Integration test - deferred
    }
}
