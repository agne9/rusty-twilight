use std::{
    collections::HashMap,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct WarningEntry {
    pub warned_at: u64,
    pub moderator_id: u64,
    pub reason: String,
}

#[derive(Clone, Copy, Debug)]
pub struct WarningRecord {
    pub warn_number: usize,
}

static WARNING_LOGS: OnceLock<RwLock<HashMap<u64, Vec<WarningEntry>>>> = OnceLock::new();

fn warning_store() -> &'static RwLock<HashMap<u64, Vec<WarningEntry>>> {
    WARNING_LOGS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Record a warning for a target user and return the new warning number.
pub async fn record_warning(user_id: u64, moderator_id: u64, reason: &str) -> WarningRecord {
    let warned_at = now_unix_secs();

    let entry = WarningEntry {
        warned_at,
        moderator_id,
        reason: reason.to_owned(),
    };

    let mut store = warning_store().write().await;
    let entries = store.entry(user_id).or_default();
    entries.push(entry);

    WarningRecord {
        warn_number: entries.len(),
    }
}

/// Return warning entries for a target user in the inclusive [since, now] range.
pub async fn warnings_since(user_id: u64, since: u64) -> Vec<WarningEntry> {
    let store = warning_store().read().await;
    let mut entries = store
        .get(&user_id)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.warned_at >= since)
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.warned_at);
    entries
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
