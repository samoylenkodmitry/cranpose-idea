#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use cranpose::{AppLauncher, embed::EmbedEndpoint};
use cranpose_intellij_ui::ToolWindow;

fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--dev-run") {
        let result = args
            .next()
            .ok_or_else(|| "missing development options".to_owned())
            .and_then(|options| serde_json::from_str(&options).map_err(|error| error.to_string()))
            .and_then(|options| {
                cranpose_dev_runner::runner::run(options).map_err(|error| format!("{error:#}"))
            });
        if let Err(error) = result {
            eprintln!(
                "{}",
                serde_json::json!({"cranposeDev": "error", "message": error})
            );
            std::process::exit(1);
        }
        return;
    }
    let content: fn() = if std::env::var_os("CRANPOSE_STUDIO").is_some() {
        cranpose_intellij_ui::studio::PreviewStudio
    } else {
        ToolWindow
    };
    let launcher = AppLauncher::new()
        .with_title("Cranpose Tool Window")
        .with_size(420, 760);
    match EmbedEndpoint::from_env() {
        Some(endpoint) => launcher.run_embedded(endpoint, content),
        #[cfg(feature = "desktop")]
        None => launcher.run(content),
        #[cfg(not(feature = "desktop"))]
        None => {
            eprintln!(
                "start this binary from the IDE plugin, or build it with the `desktop` feature"
            );
            std::process::exit(2)
        }
    }
}
