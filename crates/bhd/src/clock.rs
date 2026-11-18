//! Wall-clock helper for `bhd`.

use std::time::{SystemTime, UNIX_EPOCH};

use bh_core::Timestamp;

/// The current time in whole Unix seconds.
pub fn now() -> Timestamp {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    Timestamp::from_secs(secs)
}
