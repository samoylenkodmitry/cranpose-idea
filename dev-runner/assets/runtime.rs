#[cfg(not(debug_assertions))]
compile_error!("Cranpose hot reload is restricted to private debug builds");

use cranpose_dev_runtime as shared;
pub use shared::literal;
static LIVE_RECEIVER: std::sync::OnceLock<crate::__cranpose_api::HostMessageObserver> = std::sync::OnceLock::new();

pub fn call<A, R>(arguments: A, body: impl FnMut(A) -> R) -> R {
    shared::HotFn::current(body).call((arguments,))
}

pub fn observe_patch() {
    shared::connect();
    if shared::claim_host_receiver() {
        let observer = crate::__cranpose_api::observe_host_messages("cranpose.dev.values", |payload| {
            let result = shared::apply_values(payload);
            crate::__cranpose_api::send_to_host("cranpose.dev.values.result", &result);
        });
        let _ = LIVE_RECEIVER.set(observer);
    }
    let observed = crate::__cranpose_api::rememberMutableStateOf(shared::generation);
    crate::__cranpose_api::LaunchedEffectAsync((), move |_| Box::pin(async move {
        loop {
            crate::__cranpose_api::delay(std::time::Duration::from_millis(40)).await;
            let generation = shared::generation();
            if generation != observed.get() { observed.set(generation); }
            if shared::claim_report(generation) {
                crate::__cranpose_api::send_to_host("cranpose.dev.applied", &format!("{{\"generation\":{generation},\"pid\":{}}}", std::process::id()));
            }
        }
    }));
    let _ = observed.get();
}
