//! Native stability analysis and badge descriptions; the IDE adapter only places them.
use cranpose_stability::{
    Effect, Stability,
    project::{ProjectRequest, analyze_project},
};
use serde_json::{Value, json};
use std::io::{Read, Write};

/// Analyze disk context with unsaved editor overlays and describe inline badges.
pub fn analyze_request(request: &ProjectRequest) -> Result<Value, String> {
    let report = analyze_project(request).map_err(|e| format!("{e:#}"))?;
    let mut files = std::collections::BTreeMap::<String, Vec<Value>>::new();
    for function in &report.composables {
        let badges = files.entry(function.path.clone()).or_default();
        for parameter in &function.parameters {
            let (label, tone) = match parameter.effect {
                Effect::SkippingDisabled => (
                    "no skip",
                    if parameter.rule.is_some() {
                        "warning"
                    } else {
                        "muted"
                    },
                ),
                Effect::SharedMutation => ("shared mutation", "danger"),
                Effect::InvalidParameter => ("incompatible", "danger"),
                _ => match parameter.stability {
                    Stability::Stable => ("stable", "stable"),
                    Stability::Unstable => ("unstable", "warning"),
                    Stability::Unknown => ("unknown", "muted"),
                    Stability::Incompatible => ("incompatible", "danger"),
                },
            };
            let mut detail = format!(
                "{} · {}\n{}",
                function.name, parameter.name, parameter.reason
            );
            if let Some(resolved) = &parameter.resolved_type {
                detail.push_str(&format!("\nType: {resolved}"));
            }
            detail.push_str(&format!("\n\n{}", parameter.advice));
            if let Some(reason) = &parameter.suppressed {
                detail.push_str(&format!("\n\nAllowed: {reason}"));
            }
            badges.push(json!({
                "start":parameter.location.utf16_start,"end":parameter.location.utf16_end,
                "label":label,"tone":tone,"detail":detail,"rule":parameter.rule,
                "suppressed":parameter.suppressed.is_some()
            }));
        }
    }
    Ok(
        json!({"schemaVersion":1,"files":files.into_iter().map(|(path,badges)|json!({"path":path,"badges":badges})).collect::<Vec<_>>(),
        "diagnostics":report.diagnostics}),
    )
}

/// Read one request and write one response without starting a GPU or application window.
pub fn stdio() {
    let result = (|| {
        let mut input = String::new();
        std::io::stdin()
            .take(16 * 1024 * 1024 + 1)
            .read_to_string(&mut input)
            .map_err(|e| e.to_string())?;
        if input.len() > 16 * 1024 * 1024 {
            return Err("Stability request exceeds 16 MiB".to_string());
        }
        let request: ProjectRequest = serde_json::from_str(&input).map_err(|e| e.to_string())?;
        analyze_request(&request)
    })();
    let value = result.unwrap_or_else(|error| json!({"schemaVersion":1,"error":error}));
    if let Ok(encoded) = serde_json::to_vec(&value) {
        let _ = std::io::stdout().write_all(&encoded);
    }
}
