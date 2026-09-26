fn main() -> anyhow::Result<()> {
    cranpose_ide_tools::run_cli(cranpose_ide_tools::BuildConfig {
        root: std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace")
            .to_path_buf(),
        plugin_id: "dev.cranpose.idea".into(),
        directory: "cranpose-idea".into(),
        host_package: "cranpose-ide-host".into(),
    })
}
