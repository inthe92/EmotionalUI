//! Read-only hardware evidence without acquiring DRM, audio or input devices.
use emotional_services::system::Runtime;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::start(false);
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(15) {
        if let Some(snapshot) = runtime.latest() {
            // This type contains no SSIDs, addresses, credentials or shell output.
            println!("SYSTEM_READ_ONLY {snapshot:?}");
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err("System snapshot unavailable within diagnostic deadline".into())
}
