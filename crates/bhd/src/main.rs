//! `bhd`: the bulkhead supervisor daemon.
//!
//! Unit 01 lands the stub only. The tokio loop, event sources and socket server
//! arrive in later units; this binary exists so the workspace and protocol
//! version are exercised end to end.

fn main() {
    println!(
        "bhd {} (protocol {}.{})",
        env!("CARGO_PKG_VERSION"),
        bh_core::protocol::PROTOCOL_MAJOR,
        bh_core::protocol::PROTOCOL_MINOR,
    );
}
