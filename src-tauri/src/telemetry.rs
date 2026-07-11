use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use serde::{Serialize, Deserialize};

static ERROR_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub category: String,
    pub message: String,
    pub timestamp: i64,
}

lazy_static::lazy_static! {
    static ref ERROR_LOG: Mutex<Vec<ErrorEvent>> = Mutex::new(Vec::new());
}

pub fn record_error(category: &str, message: &str) {
    ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
    let event = ErrorEvent {
        category: category.to_string(),
        message: message.to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0),
    };

    if let Ok(mut log) = ERROR_LOG.lock() {
        log.push(event.clone());
        if log.len() > 1000 {
            log.remove(0);
        }
    }

    log::error!("[{}] {}", category, message);
}

pub fn error_count() -> u64 {
    ERROR_COUNT.load(Ordering::Relaxed)
}

pub fn get_error_log() -> Vec<ErrorEvent> {
    ERROR_LOG.lock().map(|l| l.clone()).unwrap_or_default()
}

pub fn clear_error_log() {
    if let Ok(mut log) = ERROR_LOG.lock() {
        log.clear();
    }
}
