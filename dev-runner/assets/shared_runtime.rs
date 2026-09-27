#[cfg(not(debug_assertions))]
compile_error!("Cranpose hot reload is restricted to private debug builds");

use std::sync::{Arc, Once, atomic::{AtomicU64, Ordering}};
pub use subsecond::HotFn;
pub mod values;
mod transport;
use std::sync::{Mutex, OnceLock, atomic::AtomicBool};
static VALUES: OnceLock<Mutex<values::Store>> = OnceLock::new();
static HOST_RECEIVER: AtomicBool = AtomicBool::new(false);

pub fn claim_host_receiver() -> bool { !HOST_RECEIVER.swap(true, Ordering::AcqRel) }
pub fn literal<T: values::Literal>(file: &str, schema: &str, id: usize, default: T) -> T {
    VALUES.get_or_init(Default::default).lock().expect("live values").value(file, schema, id, default)
}
pub fn apply_values(payload: String) -> String {
    if payload.len() > 1024 * 1024 {
        return serde_json::json!({"accepted":false,"error":"Live value payload exceeds 1 MiB"}).to_string();
    }
    let mut identity = serde_json::json!({});
    let result = serde_json::from_str::<values::Update>(&payload).map_err(|e| e.to_string()).and_then(|update| {
        identity = serde_json::json!({"file":update.file,"schema":update.schema,"revision":update.revision});
        VALUES.get_or_init(Default::default).lock().expect("live values").apply(&update)
    });
    match result {
        Ok(changed) => {
            if changed { GENERATION.fetch_add(1, Ordering::AcqRel); }
            identity["accepted"] = serde_json::json!(true);
            identity["changed"] = serde_json::json!(changed);
            identity["generation"] = serde_json::json!(generation());
            identity.to_string()
        }
        Err(error) => {
            identity["accepted"] = serde_json::json!(false);
            identity["error"] = serde_json::json!(error);
            identity.to_string()
        }
    }
}

static CONNECT: Once = Once::new();
static GENERATION: AtomicU64 = AtomicU64::new(0);
static REPORTED: AtomicU64 = AtomicU64::new(u64::MAX);
static COMPOSED: AtomicU64 = AtomicU64::new(u64::MAX);

// One compiler connection and generation counter across every instrumented workspace crate.
pub fn connect() {
    CONNECT.call_once(|| {
        subsecond::register_handler(Arc::new(|| { GENERATION.fetch_add(1, Ordering::AcqRel); }));
        dioxus_devtools::connect_subsecond();
        transport::receive(apply_values);
    });
}
pub fn generation() -> u64 { GENERATION.load(Ordering::Acquire) }
pub fn claim_report(generation: u64) -> bool { REPORTED.swap(generation, Ordering::AcqRel) != generation }
pub fn claim_composed(generation: u64) -> bool { COMPOSED.swap(generation, Ordering::AcqRel) != generation }
