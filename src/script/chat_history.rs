//! Process-wide Python-compatible public chat history.

use std::sync::{Mutex, OnceLock};

pub(crate) static CHAT_HISTORY: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

pub(crate) fn record_chat_history(message: &str) {
    record_chat_history_limit(message, 24);
}

pub(crate) fn record_chat_history_limit(message: &str, limit: usize) {
    let history = CHAT_HISTORY.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut history) = history.lock() {
        history.push(message.to_string());
        if history.len() > limit {
            let excess = history.len() - limit;
            history.drain(0..excess);
        }
    }
}

pub(crate) fn chat_history_snapshot() -> Vec<String> {
    CHAT_HISTORY
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .map(|history| history.clone())
        .unwrap_or_default()
}
