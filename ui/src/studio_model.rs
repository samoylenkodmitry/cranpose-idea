use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Target {
    pub package_name: String,
    pub name: String,
    pub kind: String,
    pub manifest: String,
    pub source: String,
    pub features: Vec<String>,
    pub cranpose_dependency: Option<String>,
}
impl Target {
    pub fn id(&self) -> String {
        format!("{}::{}::{}", self.manifest, self.kind, self.name)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub target: String,
    pub preview: String,
    pub width: u32,
    pub height: u32,
    pub zoom: f32,
    pub dark: bool,
    pub auto_build: bool,
    pub inspect: bool,
    pub fit: bool,
    pub hot_reload: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            target: String::new(),
            preview: String::new(),
            width: 480,
            height: 640,
            zoom: 1.0,
            dark: false,
            auto_build: true,
            inspect: false,
            fit: true,
            hot_reload: true,
        }
    }
}
impl Settings {
    pub fn normalize(&mut self) {
        self.width = self.width.clamp(120, 4096);
        self.height = self.height.clamp(120, 4096);
        self.zoom = if self.zoom.is_finite() {
            self.zoom.clamp(0.25, 2.0)
        } else {
            1.0
        };
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Preview {
    pub id: String,
    pub name: String,
    pub group: String,
    pub function: String,
    pub file: String,
    pub line: usize,
    pub width: u32,
    pub height: u32,
    pub dark: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Property {
    pub name: String,
    pub value: String,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Modifier {
    pub name: String,
    pub properties: Vec<Property>,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Source {
    pub name: String,
    pub file: String,
    pub line: usize,
    pub manifest_dir: String,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Node {
    pub id: String,
    pub parent: Option<String>,
    pub kind: String,
    pub text: Option<String>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub modifiers: Vec<Modifier>,
    pub sources: Vec<Source>,
}
impl Node {
    pub fn label(&self) -> String {
        self.text
            .as_ref()
            .filter(|text| !text.is_empty())
            .map_or_else(
                || self.kind.clone(),
                |text| {
                    format!(
                        "{} · {}",
                        self.kind,
                        text.chars().take(60).collect::<String>()
                    )
                },
            )
    }
    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.width > 0.0
            && self.height > 0.0
            && x >= self.x
            && y >= self.y
            && x < self.x + self.width
            && y < self.y + self.height
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Snapshot {
    pub schema: u32,
    pub request_id: u64,
    pub capture_micros: u64,
    pub truncated: bool,
    pub nodes: Vec<Node>,
}
impl Snapshot {
    pub fn rows(
        &self,
        query: &str,
        collapsed: &std::collections::HashSet<String>,
    ) -> Vec<cranpose_plugin_ux::tree::TreeRow> {
        use cranpose_plugin_ux::tree::{TreeEntry, filter_tree};
        let searching = !query.trim().is_empty();
        let entries: Vec<_> = self
            .nodes
            .iter()
            .map(|node| {
                // Browsing the hierarchy needs no search strings or modifier formatting.
                if !searching {
                    return TreeEntry {
                        id: &node.id,
                        parent: node.parent.as_deref(),
                        search_text: "".into(),
                    };
                }
                let mut text = format!(
                    "{} {} {}",
                    node.id,
                    node.kind,
                    node.text.as_deref().unwrap_or_default()
                );
                for source in &node.sources {
                    text.push_str(&format!(" {} {}", source.name, source.file));
                }
                for modifier in &node.modifiers {
                    text.push_str(&format!(" {}", modifier.name));
                    for property in &modifier.properties {
                        text.push_str(&format!(" {} {}", property.name, property.value));
                    }
                }
                TreeEntry {
                    id: &node.id,
                    parent: node.parent.as_deref(),
                    search_text: text.into(),
                }
            })
            .collect();
        // parse() validates all IDs and parent order before a snapshot can enter Studio.
        filter_tree(&entries, query, collapsed).unwrap_or_default()
    }

    pub fn parse(payload: &str) -> Result<Self, String> {
        let snapshot: Self = serde_json::from_str(payload).map_err(|error| error.to_string())?;
        if snapshot.schema != 2 {
            return Err("Unsupported layout inspection schema".into());
        }
        let mut seen = std::collections::HashSet::new();
        for node in &snapshot.nodes {
            if node.id.is_empty()
                || node
                    .parent
                    .as_ref()
                    .is_some_and(|parent| !seen.contains(parent))
                || !seen.insert(node.id.clone())
            {
                return Err("Invalid layout identity or parent".into());
            }
            if ![node.x, node.y, node.width, node.height]
                .iter()
                .all(|value| value.is_finite())
                || node.width < 0.0
                || node.height < 0.0
            {
                return Err("Invalid layout bounds".into());
            }
        }
        Ok(snapshot)
    }
    pub fn pick(&self, x: f32, y: f32) -> Option<&Node> {
        self.nodes.iter().rev().find(|node| node.contains(x, y))
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Diagnostic {
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: u64,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Studio {
    #[serde(skip)]
    pub viewport: Option<(u32, u32)>,
    pub settings: Settings,
    #[serde(skip)]
    pub targets: Vec<Target>,
    pub previews: Vec<Preview>,
    #[serde(skip)]
    pub snapshot: std::rc::Rc<Snapshot>,
    #[serde(skip)]
    pub initialized: bool,
    pub selected: String,
    pub collapsed: std::collections::HashSet<String>,
    pub inspector_details: bool,
    pub status: String,
    pub menu: String,
    pub pick: bool,
    pub live: bool,
    pub session: u64,
    next_session: u64,
    pending_start: bool,
    pub connected: bool,
    pub busy: bool,
    pub restart_required: bool,
    pub root: String,
    pub cache: String,
    pub source: String,
    pub requested_function: String,
    pub pid: u64,
    pub generation: u64,
    pub private_root: String,
    pub source_maps: Vec<(PathBuf, PathBuf)>,
    pub diagnostics: Vec<Diagnostic>,
    pub pan_x: f32,
    pub pan_y: f32,
}
impl Default for Studio {
    fn default() -> Self {
        Self {
            viewport: None,
            settings: Settings::default(),
            targets: vec![],
            previews: vec![],
            snapshot: Snapshot::default().into(),
            initialized: false,
            selected: String::new(),
            collapsed: Default::default(),
            inspector_details: false,
            status: "Choose an application to start a preview".into(),
            menu: String::new(),
            pick: false,
            live: true,
            session: 0,
            next_session: 0,
            pending_start: false,
            connected: false,
            busy: false,
            restart_required: false,
            root: String::new(),
            cache: String::new(),
            source: String::new(),
            requested_function: String::new(),
            pid: 0,
            generation: 0,
            private_root: String::new(),
            source_maps: Vec::new(),
            diagnostics: vec![],
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
}

impl Studio {
    /// Picking or navigating a node reveals its ancestors without losing other folds.
    pub fn select_node(&mut self, id: String) {
        let mut current = self.snapshot.nodes.iter().find(|node| node.id == id);
        if current.is_none() {
            return;
        }
        self.selected = id;
        self.inspector_details = true;
        while let Some(parent) = current.and_then(|node| node.parent.as_deref()) {
            self.collapsed.remove(parent);
            current = self.snapshot.nodes.iter().find(|node| node.id == parent);
        }
    }

    pub fn selection_path(&self) -> Vec<&Node> {
        let mut path = Vec::new();
        let mut current = self
            .snapshot
            .nodes
            .iter()
            .find(|node| node.id == self.selected);
        while let Some(node) = current {
            path.push(node);
            current = node
                .parent
                .as_ref()
                .and_then(|parent| self.snapshot.nodes.iter().find(|n| &n.id == parent));
        }
        path.reverse();
        path
    }

    pub fn target(&self) -> Option<&Target> {
        self.targets
            .iter()
            .find(|target| target.id() == self.settings.target)
            .or_else(|| self.targets.first())
    }
    pub fn start(&mut self) -> Option<Value> {
        self.pending_start = true;
        let target = self.target()?.clone();
        self.pending_start = false;
        self.settings.target = target.id();
        self.next_session += 1;
        self.session = self.next_session;
        self.busy = true;
        self.restart_required = false;
        self.status = "Preparing preview…".into();
        self.diagnostics.clear();
        self.menu.clear();
        Some(
            json!({"action": "start", "session": self.session, "preview": self.settings.preview, "options": {
                "root": self.root, "package": target.package_name, "target": target.name, "kind": target.kind,
                "cache": self.cache, "features": target.features, "hotReload": self.settings.hot_reload, "watch": self.settings.auto_build
            }}),
        )
    }
    pub fn select_preview(&mut self, id: String) -> Option<Value> {
        self.settings.preview = id;
        if let Some(preview) = self
            .previews
            .iter()
            .find(|preview| preview.id == self.settings.preview)
        {
            self.settings.width = preview.width;
            self.settings.height = preview.height;
            self.settings.dark = preview.dark;
            self.settings.normalize();
        }
        self.start()
    }
    pub fn resolve_source(&self, source: &Source) -> PathBuf {
        let path = Path::new(&source.manifest_dir).join(&source.file);
        for (private, original) in &self.source_maps {
            if let Ok(relative) = path.strip_prefix(private) {
                return original.join(relative);
            }
        }
        path.strip_prefix(&self.private_root)
            .ok()
            .filter(|_| !self.private_root.is_empty())
            .map_or_else(
                || path.clone(),
                |relative| Path::new(&self.root).join(relative),
            )
    }
    pub fn handle(&mut self, channel: &str, payload: &str) -> Vec<Value> {
        let Ok(value) = serde_json::from_str::<Value>(payload) else {
            return vec![];
        };
        let mut requests = Vec::new();
        match channel {
            "studio.viewport" => {
                if let (Some(width), Some(height)) =
                    (value["width"].as_u64(), value["height"].as_u64())
                    && width > 0
                    && height > 0
                    && width <= 65536
                    && height <= 65536
                {
                    self.viewport = Some((width as u32, height as u32));
                }
            }
            "studio.init" => {
                if let Ok(restored) = serde_json::from_value::<Self>(value["checkpoint"].clone()) {
                    let targets = std::mem::take(&mut self.targets);
                    let viewport = self.viewport;
                    *self = restored;
                    self.targets = targets;
                    self.viewport = viewport;
                }
                self.initialized = true;
                if value.get("activeSession").is_some() {
                    let active = value["activeSession"].as_u64().unwrap_or_default();
                    let candidate = value["candidateSession"].as_u64().unwrap_or_default();
                    self.session = if candidate > 0 { candidate } else { active };
                    self.next_session = self.next_session.max(active).max(candidate);
                    self.connected = active > 0;
                    self.busy = candidate > 0;
                    self.pending_start = false;
                    self.menu.clear();
                    if self.session == 0 {
                        self.pid = 0;
                        self.status = "Choose an application to start a preview".into();
                    } else if !self.busy {
                        self.status = "Preview reconnected · application state preserved".into();
                    }
                }
                self.root = value["root"].as_str().unwrap_or_default().into();
                self.cache = value["cache"].as_str().unwrap_or_default().into();
                self.source = value["source"].as_str().unwrap_or_default().into();
                if let Ok(settings) = serde_json::from_value(value["settings"].clone()) {
                    self.settings = settings;
                    self.settings.normalize();
                }
            }
            "cranpose.project" => {
                if let Ok(targets) = serde_json::from_value(value["targets"].clone()) {
                    self.targets = targets;
                }
                if self.root.is_empty() {
                    self.root = value["root"].as_str().unwrap_or_default().into();
                }
                if self.settings.target.is_empty() {
                    self.settings.target = self
                        .targets
                        .iter()
                        .find(|target| target.source == self.source)
                        .or_else(|| self.targets.first())
                        .map(Target::id)
                        .unwrap_or_default();
                }
                if self.pending_start || (!self.requested_function.is_empty() && self.session == 0)
                {
                    requests.extend(self.start());
                }
            }
            "studio.command" => match value["action"].as_str().unwrap_or_default() {
                "build" => requests.extend(self.start()),
                "showFunction" => {
                    self.requested_function = value["name"].as_str().unwrap_or_default().into();
                    if self.session == 0 {
                        requests.extend(self.start());
                    } else {
                        self.choose_requested(&mut requests);
                    }
                }
                "showBuilt" | "showTarget" => {
                    if let Ok(target) = serde_json::from_value::<Target>(value["target"].clone()) {
                        self.settings.target = target.id();
                    }
                    requests.extend(self.start());
                }
                _ => {}
            },
            "studio.child" => {
                if value["session"].as_u64() != Some(self.session) {
                    return requests;
                }
                match value["event"].as_str().unwrap_or_default() {
                    "connected" => {
                        self.snapshot = Snapshot::default().into();
                        self.selected.clear();
                        self.collapsed.clear();
                        self.inspector_details = false;
                        self.pid = 0;
                        self.generation = 0;
                        self.connected = true;
                        self.busy = false;
                        self.status = "Preview running".into();
                    }
                    "stopped" => {
                        self.connected = false;
                        self.busy = false;
                        self.status = value["message"]
                            .as_str()
                            .unwrap_or("Preview stopped")
                            .into();
                    }
                    "failed" => {
                        self.session = value["fallbackSession"].as_u64().unwrap_or_default();
                        self.connected = self.session > 0;
                        self.busy = false;
                        self.status = value["message"]
                            .as_str()
                            .unwrap_or("Preview build failed")
                            .into();
                    }
                    "log" => self.log(value["line"].as_str().unwrap_or_default()),
                    "pointer" => {
                        if self.pick {
                            let picked = self
                                .snapshot
                                .pick(
                                    value["x"].as_f64().unwrap_or_default() as f32,
                                    value["y"].as_f64().unwrap_or_default() as f32,
                                )
                                .map(|node| node.id.clone())
                                .unwrap_or_default();
                            self.select_node(picked);
                        }
                    }
                    "pan" => {
                        self.pan_x = (self.pan_x + value["x"].as_f64().unwrap_or_default() as f32)
                            .clamp(-16384.0, 0.0);
                        self.pan_y = (self.pan_y + value["y"].as_f64().unwrap_or_default() as f32)
                            .clamp(-16384.0, 0.0);
                    }
                    "message" => match value["channel"].as_str().unwrap_or_default() {
                        "cranpose.previews.v1" => {
                            if let Ok(previews) =
                                serde_json::from_str(value["payload"].as_str().unwrap_or_default())
                            {
                                self.previews = previews;
                                self.choose_requested(&mut requests);
                            }
                        }
                        "cranpose.inspector.v2.snapshot" => {
                            match Snapshot::parse(value["payload"].as_str().unwrap_or_default()) {
                                Ok(snapshot) if snapshot.request_id >= self.snapshot.request_id => {
                                    self.collapsed.retain(|id| {
                                        snapshot.nodes.iter().any(|node| &node.id == id)
                                    });
                                    if !snapshot.nodes.iter().any(|node| node.id == self.selected) {
                                        self.selected.clear();
                                    }
                                    self.snapshot = snapshot.into();
                                }
                                Err(error) => self.status = error,
                                _ => {}
                            }
                        }
                        "cranpose.dev.applied" => {
                            if let Ok(applied) = serde_json::from_str::<Value>(
                                value["payload"].as_str().unwrap_or_default(),
                            ) {
                                self.pid = applied["pid"].as_u64().unwrap_or_default();
                                self.generation =
                                    applied["generation"].as_u64().unwrap_or_default();
                                self.busy = false;
                                self.diagnostics.clear();
                                self.status = if !self.settings.hot_reload {
                                    "Preview running".into()
                                } else if self.generation == 0 {
                                    "Hot reload ready".into()
                                } else {
                                    format!("Updated · patch {} · state preserved", self.generation)
                                };
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            _ => {}
        }
        requests
    }
    fn choose_requested(&mut self, requests: &mut Vec<Value>) {
        if self.requested_function.is_empty() {
            return;
        }
        let matches: Vec<_> = self
            .previews
            .iter()
            .filter(|preview| preview.function == self.requested_function)
            .collect();
        let selected = matches
            .iter()
            .find(|preview| self.source.ends_with(&preview.file))
            .copied()
            .or_else(|| (matches.len() == 1).then(|| matches[0]));
        if let Some(preview) = selected {
            let id = preview.id.clone();
            self.requested_function.clear();
            if self.settings.preview != id {
                requests.extend(self.select_preview(id));
            }
        }
    }
    fn log(&mut self, line: &str) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            if line.contains("error") {
                self.status = line.chars().take(200).collect();
            }
            return;
        };
        if let Some(kind) = value["cranposeDev"].as_str() {
            if kind == "sourceMap" {
                if let (Some(private), Some(original)) =
                    (value["private"].as_str(), value["original"].as_str())
                {
                    self.source_maps.push((private.into(), original.into()));
                }
                return;
            }
            if kind == "workspace" {
                self.private_root = value["private"].as_str().unwrap_or_default().into();
                self.source_maps
                    .push((self.private_root.clone().into(), self.root.clone().into()));
                return;
            }
            self.status = value["message"].as_str().unwrap_or_default().into();
            self.restart_required = kind == "restartRequired";
            self.busy = matches!(kind, "preparing" | "toolchain" | "patching");
        } else if matches!(value["level"].as_str(), Some("ERROR" | "error"))
            || (value["reason"].as_str() == Some("compiler-message")
                && value["message"]["level"].as_str() == Some("error"))
        {
            let message = value["message"]["message"]
                .as_str()
                .or_else(|| value["message"].as_str())
                .unwrap_or_default();
            let detail = if value["message"].is_object() {
                &value["message"]
            } else {
                &value
            };
            let span = detail["spans"]
                .as_array()
                .and_then(|spans| spans.iter().find(|span| span["is_primary"] == true));
            let file = span
                .and_then(|span| span["file_name"].as_str())
                .map(|file| {
                    self.resolve_source(&Source {
                        manifest_dir: self.root.clone(),
                        file: file.into(),
                        ..Source::default()
                    })
                });
            let line = span
                .and_then(|span| span["line_start"].as_u64())
                .unwrap_or(1);
            self.diagnostics.push(Diagnostic {
                message: message.chars().take(300).collect(),
                file,
                line,
            });
            if self.diagnostics.len() > 30 {
                self.diagnostics.remove(0);
            }
            self.busy = false;
            self.status = "Build failed · previous preview remains available".into();
        }
    }
}

#[cfg(test)]
#[path = "tests/studio_model_tests.rs"]
mod tests;

/// Placement shared by the Cranpose layout and the native viewport clip.
#[derive(Clone, Copy, Debug)]
pub struct StudioLayout {
    pub top: f32,
    pub stage_width: f32,
    pub stage_height: f32,
    pub inspector_width: f32,
    pub inspector_height: f32,
}
impl StudioLayout {
    pub fn new(width: f32, height: f32, inspect: bool, menu: f32, problems: f32) -> Self {
        let wide = width >= 760.0;
        let top = (if wide { 88.0 } else { 126.0 }) + menu;
        let body = (height - top - problems - 28.0).max(0.0);
        let inspector_width = if inspect && wide {
            (width * 0.4).clamp(320.0, 420.0)
        } else {
            0.0
        };
        let inspector_height = if inspect && !wide {
            (height * 0.4).clamp(180.0, 380.0).min(body * 0.6)
        } else {
            0.0
        };
        Self {
            top,
            stage_width: width - inspector_width,
            stage_height: body - inspector_height,
            inspector_width,
            inspector_height,
        }
    }
}
