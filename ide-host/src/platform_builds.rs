//! The IDE invokes the same Rust library as the standalone local builder.
use anyhow::{Result, bail};
use cranpose_build::{Host, Platform, Request, engine, tools};
use cranpose_host::{
    jobs,
    jvm::{J, O},
};
use serde::Deserialize;
use serde_json::json;

const CHANNEL: &str = "cranpose.build";
#[derive(Deserialize)]
#[serde(default)]
struct Action {
    action: String,
    platform: Platform,
    manifest: String,
    package: String,
    binary: String,
    release: bool,
    device: String,
}
impl Default for Action {
    fn default() -> Self {
        Self {
            action: String::new(),
            platform: Host::default().native(),
            manifest: String::new(),
            package: String::new(),
            binary: String::new(),
            release: false,
            device: String::new(),
        }
    }
}
pub fn handle(j: &mut J<'_>, project: &O, channel: &str, payload: &str) -> Result<bool> {
    if channel != CHANNEL {
        return Ok(false);
    }
    let result = dispatch(j, project, payload);
    if let Err(error) = result {
        jobs::publish(
            j,
            project,
            CHANNEL,
            &json!({"type":"job_finished","error":format!("{error:#}"),"cancelled":false}),
        )?;
    }
    Ok(true)
}
fn dispatch(j: &mut J<'_>, project: &O, payload: &str) -> Result<()> {
    let action: Action = serde_json::from_str(payload)?;
    if action.action == "cancel" {
        return jobs::cancel(j, project);
    }
    if matches!(action.action.as_str(), "build" | "run") {
        let manager = j.static_obj(
            "com/intellij/openapi/fileEditor/FileDocumentManager",
            "getInstance",
            "()Lcom/intellij/openapi/fileEditor/FileDocumentManager;",
            &[],
        )?;
        j.void(&manager, "saveAllDocuments", "()V", &[])?;
    }
    jobs::start(j, project, CHANNEL, move |context| {
        let manifest = if action.manifest.is_empty() {
            context.root.join("Cargo.toml")
        } else {
            context.root.join(&action.manifest)
        };
        let mut request = Request::new(manifest, action.platform);
        request.package = (!action.package.is_empty()).then_some(action.package);
        request.binary = (!action.binary.is_empty()
            && !matches!(action.platform.os(), "android" | "ios"))
        .then_some(action.binary);
        request.release = action.release;
        request.device = (!action.device.is_empty()).then_some(action.device);
        let emit = |event| {
            if let Ok(value) = serde_json::to_value(event) {
                context.send(&value);
            }
        };
        match action.action.as_str() {
            "doctor" => context.send(&json!({"type":"doctor","report":tools::doctor(action.platform, &context.cancel)})),
            "setup" => tools::setup(action.platform, &context.cancel, |text| context.send(&json!({"type":"log","text":text})))?,
            "devices" => context.send(&json!({"type":"devices","text":engine::devices(action.platform, &context.cancel)?})),
            "plan" => context.send(&json!({"type":"plan","value":engine::plan(&request, &context.cancel)?})),
            "build" | "run" => {
                let artifact = engine::build(&request, &context.cancel, emit)?;
                if action.action == "run" { engine::run(&artifact, request.device.as_deref(), &context.cancel, emit)?; }
            }
            _ => bail!("Unknown platform operation"),
        }
        Ok(())
    })
}
