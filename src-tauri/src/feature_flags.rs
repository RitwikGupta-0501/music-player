use std::sync::Mutex;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FeatureFlag {
    ShuffleMode,
    ReorderQueue,
    VirtualScrolling,
    PersistentQueue,
}

pub struct FeatureFlags {
    enabled: Mutex<Vec<FeatureFlag>>,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureFlags {
    pub fn new() -> Self {
        FeatureFlags {
            enabled: Mutex::new(vec![
                FeatureFlag::ShuffleMode,
                FeatureFlag::ReorderQueue,
                FeatureFlag::VirtualScrolling,
                FeatureFlag::PersistentQueue,
            ]),
        }
    }

    pub fn is_enabled(&self, flag: FeatureFlag) -> bool {
        self.enabled
            .lock()
            .map(|flags| flags.contains(&flag))
            .unwrap_or(false)
    }

    pub fn enable(&self, flag: FeatureFlag) {
        if let Ok(mut flags) = self.enabled.lock() {
            if !flags.contains(&flag) {
                flags.push(flag);
            }
        }
    }

    pub fn disable(&self, flag: FeatureFlag) {
        if let Ok(mut flags) = self.enabled.lock() {
            flags.retain(|f| f != &flag);
        }
    }

    pub fn get_enabled_flags(&self) -> Vec<FeatureFlag> {
        self.enabled.lock().map(|f| f.clone()).unwrap_or_default()
    }
}

lazy_static::lazy_static! {
    pub static ref FEATURE_FLAGS: FeatureFlags = FeatureFlags::new();
}
