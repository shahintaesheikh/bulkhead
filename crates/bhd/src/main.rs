//! `bhd`: the bulkhead supervisor daemon.

use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();

    let paths = bhd::paths::Paths::discover()?;
    let batch = std::env::var("BULKHEAD_BATCH_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(30));

    tracing::info!(
        "bhd starting (home {}, batch {batch:?})",
        paths.home.display()
    );
    bhd::run(paths, batch).await
}
