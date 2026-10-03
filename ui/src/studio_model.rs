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
    /// Start the preview when the panel opens and the application is known.
    pub auto_start: bool,
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
            auto_start: true,
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
    pub recompositions: Option<u64>,
}
impl Source {
    fn same_origin(&self, other: &Self) -> bool {
        self.name == other.name
            && self.file == other.file
            && self.line == other.line
            && self.manifest_dir == other.manifest_dir
    }

    pub fn label(&self) -> String {
        let name = self
            .name
            .strip_prefix("__cranpose_call:")
            .unwrap_or(&self.name);
        let mut label = format!("{name} :{}", self.line);
        if let Some(count) = self.recompositions {
            let unit = if count == 1 {
                "recomposition"
            } else {
                "recompositions"
            };
            label.push_str(&format!(" · {count} {unit}"));
        }
        label
    }
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
    pub fn recompositions(&self) -> Option<u64> {
        self.sources
            .iter()
            .rev()
            .find_map(|source| source.recompositions)
    }

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecompositionInstance {
    instance_id: u64,
    #[serde(flatten)]
    source: Source,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Recompositions {
    schema: u32,
    request_id: u64,
    instances: Vec<RecompositionInstance>,
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
    pub fn hits(&self, x: f32, y: f32) -> impl Iterator<Item = &Node> {
        self.nodes
            .iter()
            .rev()
            .filter(move |node| node.contains(x, y))
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PickMenu {
    id: String,
    session: u64,
    snapshot: std::rc::Rc<Snapshot>,
    nodes: Vec<String>,
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
    inspection_sequence: cranpose_plugin_ux::delivery::SequenceGate,
    #[serde(skip)]
    recomposition_sequence: cranpose_plugin_ux::delivery::SequenceGate,
    #[serde(skip)]
    inspection_fresh: bool,
    #[serde(skip)]
    pub initialized: bool,
    pub selected: String,
    pub collapsed: std::collections::HashSet<String>,
    pub inspector_details: bool,
    pub status: String,
    pub setup: String,
    pub menu: String,
    pub pick: bool,
    #[serde(skip)]
    pick_sequence: u64,
    #[serde(skip)]
    pick_menu: Option<PickMenu>,
    pub live: bool,
    pub session: u64,
    next_session: u64,
    pub pending_start: bool,
    /// The user stopped the preview; it does not start again on its own.
    pub stopped: bool,
    /// The project's Cargo status while its applications are being read.
    #[serde(skip)]
    pub project_status: String,
    /// The project's Cargo metadata has been read at least once.
    #[serde(skip)]
    pub project_read: bool,
    /// The crate the preview build is compiling, and how many it has compiled.
    #[serde(skip)]
    pub build_step: String,
    #[serde(skip)]
    pub built: u32,
    pub connected: bool,
    pub busy: bool,
    pub restart_required: bool,
    /// A replacement session is building after an edit the running preview cannot load.
    pub rebuilding: bool,
    pub rebuild_reason: String,
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
            inspection_sequence: Default::default(),
            recomposition_sequence: Default::default(),
            inspection_fresh: false,
            initialized: false,
            selected: String::new(),
            collapsed: Default::default(),
            inspector_details: false,
            status: String::new(),
            setup: String::new(),
            menu: String::new(),
            pick: false,
            pick_sequence: 0,
            pick_menu: None,
            live: true,
            session: 0,
            next_session: 0,
            pending_start: false,
            stopped: false,
            project_status: String::new(),
            project_read: false,
            build_step: String::new(),
            built: 0,
            connected: false,
            busy: false,
            restart_required: false,
            rebuilding: false,
            rebuild_reason: String::new(),
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
    fn invalidate_inspection(&mut self) {
        self.inspection_fresh = false;
        self.inspection_sequence.discard_pending();
        self.pick_menu = None;
    }

    pub fn set_inspecting(&mut self, inspecting: bool) {
        if self.settings.inspect != inspecting {
            self.settings.inspect = inspecting;
            self.invalidate_inspection();
        }
    }

    pub fn set_live_inspection(&mut self, live: bool) {
        if self.live != live {
            self.live = live;
            self.invalidate_inspection();
        }
    }

    pub fn set_picking(&mut self, pick: bool) {
        if !pick {
            self.pick_menu = None;
        }
        if pick {
            self.set_live_inspection(true);
        }
        if self.pick != pick && !self.settings.inspect {
            self.invalidate_inspection();
        }
        self.pick = pick;
    }

    fn has_current_inspection(&self) -> bool {
        self.connected && (self.settings.inspect || self.pick) && self.live && self.inspection_fresh
    }

    pub fn picking(&self) -> bool {
        self.pick && self.has_current_inspection()
    }

    pub fn selected_bounds(&self) -> Option<Value> {
        if !self.has_current_inspection() {
            return None;
        }
        self.snapshot
            .nodes
            .iter()
            .find(|node| node.id == self.selected)
            .map(|node| json!({"x":node.x,"y":node.y,"width":node.width,"height":node.height}))
    }

    pub fn layout(&self, width: f32, height: f32) -> StudioLayout {
        let height = height.max(240.0);
        let custom_height = if self.menu == "size" { 40.0 } else { 0.0 };
        let problems_height = if self.diagnostics.is_empty() {
            0.0
        } else {
            (30.0 + 26.0 * self.diagnostics.len().min(4) as f32).min(height * 0.3)
        };
        StudioLayout::new(
            width.max(300.0),
            height,
            self.settings.inspect,
            custom_height,
            problems_height,
        )
    }

    pub fn preview_frame(&self, layout: &StudioLayout) -> PreviewFrame {
        let scale = if self.settings.fit {
            ((layout.stage_width - 48.0) / self.settings.width as f32)
                .min((layout.stage_height - 64.0) / self.settings.height as f32)
                .clamp(0.05, 1.0)
        } else {
            self.settings.zoom
        };
        let width = self.settings.width as f32 * scale;
        let height = self.settings.height as f32 * scale;
        let x = if width <= layout.stage_width {
            (layout.stage_width - width) * 0.5
        } else {
            self.pan_x.clamp(layout.stage_width - width, 0.0)
        };
        let y = layout.top
            + if height <= layout.stage_height {
                ((layout.stage_height - height) * 0.5)
                    .max(28.0f32.min(layout.stage_height - height))
            } else {
                self.pan_y.clamp(layout.stage_height - height, 0.0)
            };
        PreviewFrame {
            x,
            y,
            width,
            height,
            scale,
        }
    }

    pub fn inspection_request(&self) -> Option<Value> {
        self.connected.then(|| {
            json!({"action":"message", "session":self.session,
            "channel":"cranpose.inspector.v2.request",
            // Cranpose's v2 request is a decimal u64, not a JSON object.
            "payload":self.inspection_sequence.next_request().to_string()})
        })
    }

    pub fn recomposition_request(&self) -> Option<Value> {
        self.connected.then(|| {
            json!({"action":"message", "session":self.session,
            "channel":"cranpose.recompositions.v1.request",
            "payload":self.recomposition_sequence.next_request().to_string()})
        })
    }

    fn editor_recompositions(&self, payload: &str) -> Option<Value> {
        let snapshot: Recompositions = serde_json::from_str(payload).ok()?;
        if !self.connected
            || snapshot.schema != 1
            || !self.recomposition_sequence.accept(snapshot.request_id)
        {
            return None;
        }
        let mut seen = std::collections::HashSet::new();
        let mut totals = std::collections::BTreeMap::new();
        for instance in snapshot.instances {
            let source = instance.source;
            let Some(count) = source.recompositions else {
                continue;
            };
            if !seen.insert(instance.instance_id) || source.line == 0 {
                continue;
            }
            let file = self.resolve_source(&source);
            let total = totals
                .entry((file, source.name, source.line))
                .or_insert((0u64, 0u64));
            total.0 = total.0.saturating_add(count);
            total.1 = total.1.saturating_add(1);
        }
        let rows: Vec<_> = totals.into_iter().map(|((file, name, line), (count, instances))| {
            json!({"file":file,"name":name,"line":line,"recompositions":count,"instances":instances})
        }).collect();
        Some(json!({"action":"recompositions", "session":self.session,"rows":rows}))
    }

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

    /// Prefer the exact application call recorded by the private preview build,
    /// then an application composable definition. Framework internals are never
    /// the automatic destination when the application supplied an origin.
    pub fn selected_source_request(&self) -> Option<Value> {
        let node = self
            .snapshot
            .nodes
            .iter()
            .find(|node| node.id == self.selected)?;
        let source = self.node_source(node)?;
        Some(
            json!({"action":"navigate","file":self.resolve_source(source),"line":source.line,"reveal":true}),
        )
    }

    fn node_source<'a>(&self, node: &'a Node) -> Option<&'a Source> {
        if self.root.is_empty() {
            return None;
        }
        let local = |source: &&Source| {
            std::path::Path::new(&self.resolve_source(source)).starts_with(&self.root)
        };
        node.sources
            .iter()
            .rev()
            .find(|source| source.name.starts_with("__cranpose_call:") && local(source))
            .or_else(|| node.sources.iter().rev().find(local))
    }

    fn pick_at(&mut self, x: f32, y: f32) -> Option<Value> {
        self.pick_menu = None;
        if !self.picking() || !x.is_finite() || !y.is_finite() {
            return None;
        }
        let hits: Vec<_> = self.snapshot.hits(x, y).collect();
        match hits.as_slice() {
            [] => None,
            [node] => {
                let id = node.id.clone();
                self.select_node(id);
                self.selected_source_request()
            }
            _ => {
                let items: Vec<_> = hits
                    .iter()
                    .map(|node| {
                        let mut label = format!(
                            "{}  ·  {} × {}",
                            node.label().replace(['\r', '\n', '\t'], " "),
                            node.width.round(),
                            node.height.round()
                        );
                        if let Some(source) = self.node_source(node) {
                            let file = Path::new(&source.file)
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy();
                            label.push_str(&format!("  ·  {file}:{}", source.line));
                        }
                        json!({"id":node.id,"label":label,"checked":node.id == self.selected})
                    })
                    .collect();
                self.pick_sequence += 1;
                let id = format!("pick:{}:{}", self.session, self.pick_sequence);
                let (width, height) = self
                    .viewport
                    .unwrap_or((self.settings.width, self.settings.height));
                let layout = self.layout(width as f32, height as f32);
                let frame = self.preview_frame(&layout);
                self.pick_menu = Some(PickMenu {
                    id: id.clone(),
                    session: self.session,
                    snapshot: self.snapshot.clone(),
                    nodes: hits.iter().map(|node| node.id.clone()).collect(),
                });
                Some(json!({"action":"menu","menu":id,"items":items,
                    "x":frame.x.trunc() + x * frame.scale,
                    "y":frame.y.trunc() + y * frame.scale + 6.0,"width":240}))
            }
        }
    }

    fn choose_pick(&mut self, menu: &str, id: &str) -> Option<Value> {
        let pending = self.pick_menu.as_ref()?;
        if pending.id != menu || pending.session != self.session || !self.picking() {
            return None;
        }
        let pending = self.pick_menu.take()?;
        if !pending.nodes.iter().any(|node| node == id) {
            return None;
        }
        let original = pending.snapshot.nodes.iter().find(|node| node.id == id)?;
        let current = self.snapshot.nodes.iter().find(|node| node.id == id)?;
        if original.kind != current.kind
            || original.sources.len() != current.sources.len()
            || !original
                .sources
                .iter()
                .zip(&current.sources)
                .all(|(a, b)| a.same_origin(b))
        {
            return None;
        }
        self.select_node(id.to_owned());
        self.selected_source_request()
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
        self.stopped = false;
        let Some(target) = self.target().cloned() else {
            self.status = if self.project_read && !self.project_status.is_empty() {
                self.project_status.clone()
            } else {
                "Reading Cargo.toml".into()
            };
            return None;
        };
        self.pending_start = false;
        self.settings.target = target.id();
        self.next_session += 1;
        self.session = self.next_session;
        self.busy = true;
        self.restart_required = false;
        self.rebuilding = false;
        self.rebuild_reason.clear();
        self.status = format!("Building {}", target.name);
        self.build_step.clear();
        self.built = 0;
        self.setup.clear();
        self.diagnostics.clear();
        self.menu.clear();
        self.pick_menu = None;
        Some(
            json!({"action": "start", "session": self.session, "preview": self.settings.preview, "options": {
                "root": self.root, "package": target.package_name, "target": target.name, "kind": target.kind,
                "cache": self.cache, "features": target.features, "hotReload": self.settings.hot_reload, "watch": self.settings.auto_build
            }}),
        )
    }
    /// Starts the preview on its own once the application is known, unless the
    /// user stopped it or a session is already running or starting.
    fn auto_start(&mut self) -> Option<Value> {
        if !self.settings.auto_start || self.stopped || self.session > 0 || !self.setup.is_empty() {
            return None;
        }
        self.start()
    }
    /// Starts a replacement after an edit the running process cannot load. The host
    /// keeps the current preview visible and interactive until the new one connects.
    fn rebuild(&mut self, reason: &str) -> Option<Value> {
        let request = self.start()?;
        self.rebuilding = true;
        self.rebuild_reason = reason.into();
        self.status = format!("Rebuilding: {reason}");
        Some(request)
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
        cranpose_plugin_ux::source_paths::resolve(
            Path::new(&source.file),
            Path::new(&source.manifest_dir),
            Path::new(&self.private_root),
            Path::new(&self.root),
            &self.source_maps,
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
                    self.rebuilding &= self.busy;
                    self.pending_start = false;
                    self.menu.clear();
                    if self.session == 0 {
                        self.pid = 0;
                        self.status.clear();
                    } else if !self.busy {
                        self.status = "Reconnected to the running preview".into();
                    }
                }
                self.root = value["root"].as_str().unwrap_or_default().into();
                self.cache = value["cache"].as_str().unwrap_or_default().into();
                self.source = value["source"].as_str().unwrap_or_default().into();
                if let Ok(settings) = serde_json::from_value(value["settings"].clone()) {
                    self.settings = settings;
                    self.settings.normalize();
                }
                requests.extend(self.auto_start());
            }
            "cranpose.project" => {
                if let Ok(targets) = serde_json::from_value(value["targets"].clone()) {
                    self.targets = targets;
                }
                self.project_status = value["status"].as_str().unwrap_or_default().into();
                self.project_read |= !value["busy"].as_bool().unwrap_or(false);
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
                } else {
                    requests.extend(self.auto_start());
                }
            }
            "studio.command" => match value["action"].as_str().unwrap_or_default() {
                // An automatic or earlier start may already be building this preview.
                "build" if !(self.busy && !self.connected && self.session > 0) => {
                    requests.extend(self.start())
                }
                "menu" => requests.extend(self.choose(
                    value["menu"].as_str().unwrap_or_default(),
                    value["item"].as_str().unwrap_or_default(),
                )),
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
                        self.pick_menu = None;
                        self.snapshot = Snapshot::default().into();
                        self.inspection_sequence = Default::default();
                        self.recomposition_sequence = Default::default();
                        self.inspection_fresh = false;
                        self.selected.clear();
                        self.collapsed.clear();
                        self.inspector_details = false;
                        self.pid = 0;
                        self.generation = 0;
                        self.connected = true;
                        self.busy = false;
                        self.build_step.clear();
                        self.status = if self.rebuilding {
                            format!("Rebuilt: {}", self.rebuild_reason)
                        } else {
                            "Running".into()
                        };
                        self.rebuilding = false;
                    }
                    "stopped" => {
                        self.pick_menu = None;
                        self.connected = false;
                        self.busy = false;
                        self.rebuilding = false;
                        self.setup = value["setup"].as_str().unwrap_or_default().into();
                        if !self.setup.is_empty() {
                            self.session = 0;
                        }
                        self.status = value["message"]
                            .as_str()
                            .unwrap_or("Preview stopped")
                            .into();
                    }
                    "failed" => {
                        self.pick_menu = None;
                        self.session = value["fallbackSession"].as_u64().unwrap_or_default();
                        self.connected = self.session > 0;
                        self.busy = false;
                        self.rebuilding = false;
                        self.status = value["message"].as_str().unwrap_or("Build failed").into();
                    }
                    "log" => requests.extend(self.log(value["line"].as_str().unwrap_or_default())),
                    "pointer" => {
                        if let (Some(x), Some(y)) = (value["x"].as_f64(), value["y"].as_f64()) {
                            requests.extend(self.pick_at(x as f32, y as f32));
                        }
                    }
                    "pan" => {
                        self.pan_x = (self.pan_x + value["x"].as_f64().unwrap_or_default() as f32)
                            .clamp(-16384.0, 0.0);
                        self.pan_y = (self.pan_y + value["y"].as_f64().unwrap_or_default() as f32)
                            .clamp(-16384.0, 0.0);
                    }
                    "message" => match value["channel"].as_str().unwrap_or_default() {
                        "cranpose.recompositions.v1.snapshot" => {
                            requests.extend(self.editor_recompositions(
                                value["payload"].as_str().unwrap_or_default(),
                            ));
                        }
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
                                Ok(snapshot)
                                    if self.inspection_sequence.accept(snapshot.request_id) =>
                                {
                                    self.inspection_fresh = true;
                                    // Request IDs and capture duration change on every poll.
                                    // Retain the shared view when the actual layout is unchanged.
                                    if snapshot.schema != self.snapshot.schema
                                        || snapshot.truncated != self.snapshot.truncated
                                        || snapshot.nodes != self.snapshot.nodes
                                    {
                                        let ids: std::collections::HashSet<_> = snapshot
                                            .nodes
                                            .iter()
                                            .map(|node| node.id.as_str())
                                            .collect();
                                        self.collapsed.retain(|id| ids.contains(id.as_str()));
                                        if !ids.contains(self.selected.as_str()) {
                                            self.selected.clear();
                                        }
                                        self.snapshot = snapshot.into();
                                    }
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
                                self.status = if !self.settings.hot_reload || self.generation == 0 {
                                    "Running".into()
                                } else {
                                    format!(
                                        "Applied {} code change{}",
                                        self.generation,
                                        if self.generation == 1 { "" } else { "s" }
                                    )
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
    /// Items for a native IDE menu, in the host's `menu` request format.
    pub fn menu_items(&self, menu: &str) -> Vec<Value> {
        let item = |id: &str, label: &str, checked: bool| json!({"id": id, "label": label, "checked": checked});
        let separated = |mut value: Value| {
            value["separator"] = json!(true);
            value
        };
        match menu {
            "target" => self
                .targets
                .iter()
                .map(|target| {
                    let mut label = target.name.clone();
                    if target.package_name != target.name {
                        label.push_str(&format!(" · {}", target.package_name));
                    }
                    if target.kind == "example" {
                        label.push_str(" · example");
                    }
                    let id = target.id();
                    item(&id, &label, id == self.settings.target)
                })
                .collect(),
            "preview" => std::iter::once(item("", "Application", self.settings.preview.is_empty()))
                .chain(self.previews.iter().enumerate().map(|(index, preview)| {
                    let label = if preview.group.is_empty() {
                        preview.name.clone()
                    } else {
                        format!("{} › {}", preview.group, preview.name)
                    };
                    let entry = item(&preview.id, &label, preview.id == self.settings.preview);
                    if index == 0 { separated(entry) } else { entry }
                }))
                .collect(),
            "size" => {
                let (width, height) = (self.settings.width, self.settings.height);
                let mut items: Vec<_> = SIZES
                    .iter()
                    .map(|(name, w, h)| {
                        item(
                            &format!("{w}x{h}"),
                            &format!("{name}  {w} × {h}"),
                            (width, height) == (*w, *h),
                        )
                    })
                    .collect();
                items.push(separated(item("rotate", "Rotate", false)));
                items.push(item("custom", "Custom size…", false));
                items
            }
            "zoom" => ZOOMS
                .iter()
                .map(|(label, zoom)| {
                    let checked = if *zoom == 0.0 {
                        self.settings.fit
                    } else {
                        !self.settings.fit && (self.settings.zoom - zoom).abs() < 0.001
                    };
                    item(&zoom.to_string(), label, checked)
                })
                .collect(),
            "more" => vec![
                item(
                    "autoStart",
                    "Start preview automatically",
                    self.settings.auto_start,
                ),
                item("hotReload", "Hot code reload", self.settings.hot_reload),
                item("autoBuild", "Rebuild on save", self.settings.auto_build),
                item("live", "Live inspection", self.live),
                separated(item("configure", "Run configuration…", false)),
                item("export", "Export PNG…", false),
                item("reveal", "Reveal source", false),
            ],
            _ => vec![],
        }
    }

    /// Apply a native menu choice. Settings that shape the running process
    /// restart the preview immediately rather than waiting for the user.
    pub fn choose(&mut self, menu: &str, item: &str) -> Vec<Value> {
        let mut requests = vec![];
        match (menu, item) {
            (menu, id) if menu.starts_with("pick:") => requests.extend(self.choose_pick(menu, id)),
            ("target", id) if self.targets.iter().any(|t| t.id() == id) => {
                if self.settings.target != id {
                    self.settings.target = id.into();
                    self.settings.preview.clear();
                    if self.session > 0 {
                        requests.extend(self.start());
                    }
                }
            }
            ("preview", id) if id.is_empty() || self.previews.iter().any(|p| p.id == id) => {
                requests.extend(self.select_preview(id.into()));
            }
            ("size", "rotate") => {
                let settings = &mut self.settings;
                (settings.width, settings.height) = (settings.height, settings.width);
            }
            ("size", "custom") => self.menu = "size".into(),
            ("size", preset) => {
                if let Some((w, h)) = preset.split_once('x')
                    && let (Ok(w), Ok(h)) = (w.parse(), h.parse())
                {
                    self.settings.width = w;
                    self.settings.height = h;
                    self.settings.normalize();
                }
            }
            ("zoom", zoom) => {
                if let Ok(zoom) = zoom.parse::<f32>() {
                    self.settings.fit = zoom == 0.0;
                    if zoom > 0.0 {
                        self.settings.zoom = zoom;
                        self.settings.normalize();
                    }
                }
            }
            ("more", "hotReload" | "autoBuild") => {
                if item == "hotReload" {
                    self.settings.hot_reload = !self.settings.hot_reload;
                } else {
                    self.settings.auto_build = !self.settings.auto_build;
                }
                if self.session > 0 {
                    requests.extend(self.start());
                }
            }
            ("more", "autoStart") => {
                self.settings.auto_start = !self.settings.auto_start;
                requests.extend(self.auto_start());
            }
            ("more", "live") => self.set_live_inspection(!self.live),
            ("more", "configure") => {
                requests.push(json!({"action":"configure","target":self.settings.target}))
            }
            ("more", "export") => requests.push(json!({"action":"export","session":self.session})),
            ("more", "reveal") => requests.extend(self.reveal_request()),
            _ => {}
        }
        requests
    }

    /// The selected node's application source, else the preview function.
    pub fn reveal_request(&self) -> Option<Value> {
        if let Some(request) = self.selected_source_request() {
            return Some(request);
        }
        if let Some(source) = self
            .snapshot
            .nodes
            .iter()
            .find(|node| node.id == self.selected)
            .and_then(|node| node.sources.last())
        {
            return Some(
                json!({"action": "navigate", "file": self.resolve_source(source), "line": source.line}),
            );
        }
        self.previews
            .iter()
            .find(|preview| preview.id == self.settings.preview)
            .map(|preview| {
                json!({"action": "navigate", "file": Path::new(&self.root).join(&preview.file), "line": preview.line})
            })
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
    fn log(&mut self, line: &str) -> Option<Value> {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            let text = strip_ansi(line);
            if let Some(unit) = text.trim_start().strip_prefix("Compiling ") {
                // "Compiling name v1.2.3 (/path)" becomes "Compiling name v1.2.3".
                self.built += 1;
                self.build_step = format!("Compiling {}", unit.split(" (").next().unwrap_or(unit));
            } else if text.contains("error") {
                self.status = text.chars().take(200).collect();
            }
            return None;
        };
        if value["message"]
            .as_str()
            .is_some_and(|message| message.contains("launching app"))
        {
            self.build_step = "Starting the application".into();
        }
        if let Some(kind) = value["cranposeDev"].as_str() {
            if kind == "sourceMap" {
                if let (Some(private), Some(original)) =
                    (value["private"].as_str(), value["original"].as_str())
                {
                    self.map_source(private.into(), original.into());
                }
                return None;
            }
            if kind == "workspace" {
                self.private_root = value["private"].as_str().unwrap_or_default().into();
                self.map_source(self.private_root.clone().into(), self.root.clone().into());
                return None;
            }
            let message = value["message"].as_str().unwrap_or_default();
            if kind == "rebuildRequired" && self.settings.auto_build {
                return self.rebuild(message);
            }
            // The replacement's progress stays under its reason; problems still show.
            if self.rebuilding && !matches!(kind, "error" | "restartRequired" | "rebuildRequired") {
                return None;
            }
            self.status = message.into();
            self.restart_required = matches!(kind, "restartRequired" | "rebuildRequired");
            self.busy = self.rebuilding || matches!(kind, "preparing" | "toolchain" | "patching");
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
            self.rebuilding = false;
            self.status = "Build failed. The previous preview is still running.".into();
        }
        None
    }
    fn map_source(&mut self, private: PathBuf, original: PathBuf) {
        // Every rebuild reports its private copy again; keep one entry per path.
        if !self
            .source_maps
            .contains(&(private.clone(), original.clone()))
        {
            self.source_maps.push((private, original));
        }
    }
}

/// Cargo and the hot-patch compiler color their output.
fn strip_ansi(line: &str) -> String {
    let mut text = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            text.push(c);
        }
    }
    text
}

#[cfg(test)]
#[path = "tests/studio_model_tests.rs"]
mod tests;

/// Toolbar height including its bottom hairline.
pub const TOOLBAR_HEIGHT: f32 = 45.0;
/// Status bar height below the stage.
pub const STATUS_HEIGHT: f32 = 26.0;
const SIZES: [(&str, u32, u32); 4] = [
    ("Compact", 360, 640),
    ("Phone", 480, 760),
    ("Tablet", 800, 600),
    ("Desktop", 1280, 800),
];
const ZOOMS: [(&str, f32); 6] = [
    ("Fit", 0.0),
    ("50%", 0.5),
    ("75%", 0.75),
    ("100%", 1.0),
    ("150%", 1.5),
    ("200%", 2.0),
];

/// Placement shared by the Cranpose layout and the native viewport clip.
#[derive(Clone, Copy, Debug)]
pub struct StudioLayout {
    pub top: f32,
    pub stage_width: f32,
    pub stage_height: f32,
    pub inspector_width: f32,
    pub inspector_height: f32,
    pub custom_height: f32,
    pub problems_height: f32,
}

/// Application placement in Studio coordinates, shared with cursor-anchored picking.
pub struct PreviewFrame {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
}

impl StudioLayout {
    pub fn new(width: f32, height: f32, inspect: bool, menu: f32, problems: f32) -> Self {
        let wide = width >= 760.0;
        let top = TOOLBAR_HEIGHT + menu;
        let body = (height - top - problems - STATUS_HEIGHT).max(0.0);
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
            custom_height: menu,
            problems_height: problems,
        }
    }
}
