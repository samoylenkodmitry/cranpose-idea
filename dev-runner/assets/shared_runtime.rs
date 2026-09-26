#[cfg(not(debug_assertions))]
compile_error!("Cranpose hot reload is restricted to private debug builds");

use std::sync::{Arc, Once, atomic::{AtomicU64, Ordering}};
pub use subsecond::HotFn;

static CONNECT: Once = Once::new();
static GENERATION: AtomicU64 = AtomicU64::new(0);
static REPORTED: AtomicU64 = AtomicU64::new(u64::MAX);

// One compiler connection and generation counter across every instrumented workspace crate.
pub fn connect() {
    CONNECT.call_once(|| {
        subsecond::register_handler(Arc::new(|| { GENERATION.fetch_add(1, Ordering::AcqRel); }));
        dioxus_devtools::connect_subsecond();
    });
}
pub fn generation() -> u64 { GENERATION.load(Ordering::Acquire) }
pub fn claim_report(generation: u64) -> bool { REPORTED.swap(generation, Ordering::AcqRel) != generation }
