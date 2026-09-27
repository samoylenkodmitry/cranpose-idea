fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    cranpose_dev_runner::linker::dispatch()?;
    if let Some(options) = std::env::args().nth(1).filter(|arg| arg.starts_with('{')) {
        return cranpose_dev_runner::runner::run(serde_json::from_str(&options)?);
    }
    let mut args = std::env::args_os().skip(1);
    let source = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("expected Rust source path"))?;
    let destination = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("expected destination path"))?;
    let original = std::fs::read_to_string(source)?;
    let instrumented = cranpose_dev_runner::instrumentation::instrument(&original)?;
    std::fs::write(destination, instrumented)?;
    Ok(())
}
