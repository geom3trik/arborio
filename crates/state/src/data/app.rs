use arborio_maploader::action::{MapAction, RoomAction, StylegroundSelection};
use arborio_maploader::map_struct::{CelesteMap, CelesteMapLevel};
use arborio_modloader::aggregate::ModuleAggregate;
use arborio_modloader::module::{CelesteModule, MapPath, ModuleID, CELESTE_MODULE_ID};
use arborio_modloader::selectable::{
    DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable,
};
use arborio_utils::units::*;
use arborio_utils::vizia::prelude::*;
use parking_lot::Mutex;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time;

use crate::auto_saver::AutoSaver;
use crate::data::config_editor::{
    AnyConfig, ConfigSearchFilter, ConfigSearchResult, ConfigSearchType, SearchScope,
};
use crate::data::project_map::{MapEvent, MapState, ProjectEvent};
use crate::data::selection::AppSelection;
use crate::data::tabs::{AppTab, MapTab};
use crate::data::{AppConfig, ArborioRecord, EventPhase, Layer, MapID, Progress};
use crate::tools::{Tool, ToolSpec};

#[derive(Lens)]
pub struct AppState {
    pub config: AutoSaver<AppConfig>,

    pub modules: HashMap<ModuleID, CelesteModule>,
    pub modules_lookup: HashMap<String, ModuleID>,
    pub modules_version: u32,
    pub omni_palette: ModuleAggregate,
    pub loaded_maps: HashMap<MapID, MapState>,
    pub loaded_maps_lookup: HashMap<MapPath, MapID>,

    pub current_tab: usize,
    pub tabs: Vec<AppTab>,
    pub poison_tab: usize,

    pub current_toolspec: ToolSpec,
    pub current_tool: RefCell<Option<Box<dyn Tool>>>,
    pub current_layer: Layer,
    pub current_fg_tile: TileSelectable,
    pub current_bg_tile: TileSelectable,
    pub current_entity: EntitySelectable,
    pub current_trigger: TriggerSelectable,
    pub current_decal: DecalSelectable,
    pub current_objtile: u32,
    pub objtiles_transform: MapToScreen,

    pub draw_interval: f32,
    pub snap: bool,

    pub last_draw: RefCell<time::Instant>, // mutable to draw
    pub progress: Progress,
    pub logs: Vec<ArborioRecord>,
    pub error_message: String,
}

#[derive(Debug)]
pub enum AppEvent {
    Log {
        message: ArborioRecord,
    },
    Progress {
        progress: Progress,
    },
    SetClipboard {
        contents: String,
    },
    SetConfigPath {
        path: PathBuf,
    },
    SetLastPath {
        path: PathBuf,
    },
    SetModules {
        modules: Mutex<HashMap<ModuleID, CelesteModule>>,
    },
    OpenModuleOverviewTab {
        module: ModuleID,
    },
    OpenMap {
        path: MapPath,
    },
    LoadMap {
        path: MapPath,
        map: RefCell<Option<Box<CelesteMap>>>,
    },
    OpenInstallationTab,
    OpenConfigEditorTab,
    OpenLogsTab,
    SelectTab {
        idx: usize,
    },
    CloseTab {
        idx: usize,
    },
    NewMod,
    MovePreview {
        tab: usize,
        pos: MapPointStrict,
    },
    Pan {
        tab: usize,
        delta: MapVectorPrecise,
    },
    Zoom {
        tab: usize,
        delta: f32,
        focus: MapPointPrecise,
    },
    PanObjectTiles {
        delta: MapVectorPrecise,
    },
    ZoomObjectTiles {
        delta: f32,
        focus: MapPointPrecise,
    },
    SelectTool {
        spec: ToolSpec,
    },
    SelectSearchScope {
        tab: usize,
        scope: SearchScope,
    },
    SelectSearchType {
        tab: usize,
        ty: ConfigSearchType,
    },
    SelectSearchFilter {
        tab: usize,
        filter: ConfigSearchFilter,
    },
    SelectSearchFilterAttributes {
        tab: usize,
        filter: String,
    },
    PopulateConfigSearchResults {
        tab: usize,
        results: Vec<ConfigSearchResult>,
    },
    SelectConfigSearchResult {
        tab: usize,
        idx: usize,
    },
    EditConfig {
        tab: usize,
        config: AnyConfig,
    },
    SetConfigErrorMessage {
        tab: usize,
        message: String,
    },
    SelectStyleground {
        tab: usize,
        styleground: Option<StylegroundSelection>,
    },
    SelectRoom {
        tab: usize,
        idx: usize,
    },
    SelectLayer {
        layer: Layer,
    },
    SelectPaletteTile {
        fg: bool,
        tile: TileSelectable,
    },
    SelectPaletteObjectTile {
        tile: u32,
    },
    SelectPaletteEntity {
        entity: EntitySelectable,
    },
    SelectPaletteTrigger {
        trigger: TriggerSelectable,
    },
    SelectPaletteDecal {
        decal: DecalSelectable,
    },
    SelectObject {
        tab: usize,
        selection: Option<AppSelection>,
    },
    MapEvent {
        map: Option<MapID>,
        event: MapEvent,
    },
    ProjectEvent {
        project: Option<ModuleID>,
        event: ProjectEvent,
    },
}

#[derive(Debug)]
#[non_exhaustive]
#[allow(clippy::enum_variant_names)]
pub enum AppInternalEvent {
    SelectMeEntity { id: i32, trigger: bool },
    SelectMeDecal { id: u32, fg: bool },
    SelectMeRoom { idx: usize },
}

impl Model for AppState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| {
            self.apply(cx, app_event);
        });
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> AppState {
        let mut cfg: AppConfig = confy::load("arborio").unwrap_or_default();
        if !cfg
            .celeste_root
            .as_ref()
            .map(|root| root.is_dir())
            .unwrap_or_default()
        {
            cfg.celeste_root = None;
        }
        let cfg = AutoSaver::new(cfg, |cfg: &mut AppConfig| {
            confy::store("arborio", &cfg)
                .unwrap_or_else(|e| panic!("Failed to save config file: {}", e));
        });

        AppState {
            config: cfg,
            current_tab: 0,
            poison_tab: usize::MAX,
            tabs: vec![AppTab::CelesteOverview],
            loaded_maps: HashMap::new(),
            loaded_maps_lookup: HashMap::new(),
            current_toolspec: ToolSpec::Selection,
            current_tool: RefCell::new(None),
            current_fg_tile: TileSelectable::default(),
            current_bg_tile: TileSelectable::default(),
            current_entity: EntitySelectable::default(),
            current_trigger: TriggerSelectable::default(),
            current_decal: DecalSelectable::default(),
            draw_interval: 4.0,
            snap: true,
            last_draw: RefCell::new(time::Instant::now()),
            current_layer: Layer::FgTiles,
            current_objtile: 0,
            objtiles_transform: MapToScreen::identity(),

            modules: HashMap::new(),
            modules_lookup: HashMap::new(),
            modules_version: 0,
            omni_palette: ModuleAggregate::new(
                &HashMap::new(),
                &HashMap::new(),
                &CelesteMap::default(),
                *CELESTE_MODULE_ID,
                false,
            ),
            progress: Progress {
                progress: 100,
                status: "".to_owned(),
            },
            logs: vec![],
            error_message: "".to_owned(),
        }
    }

    // a debugging stopgap
    pub fn map_tab_check(&self) -> bool {
        matches!(self.tabs.get(self.current_tab), Some(AppTab::Map(_)))
    }

    // intended mainly for use in tools. can we maybe do better?
    pub fn map_tab_unwrap(&self) -> &MapTab {
        if let Some(AppTab::Map(result)) = self.tabs.get(self.current_tab) {
            result
        } else {
            panic!("misuse of map_tab_unwrap");
        }
    }

    pub fn current_project_id(&self) -> Option<ModuleID> {
        match self.tabs.get(self.current_tab) {
            Some(AppTab::ProjectOverview(id)) => Some(*id),
            Some(AppTab::Map(maptab)) => {
                Some(self.loaded_maps.get(&maptab.id).unwrap().path.module)
            }
            _ => None,
        }
    }

    pub fn current_palette_unwrap(&self) -> &ModuleAggregate {
        if let Some(AppTab::Map(result)) = self.tabs.get(self.current_tab) {
            &self
                .loaded_maps
                .get(&result.id)
                .expect("stale reference")
                .palette
        } else {
            panic!("misuse of current_palette_unwrap");
        }
    }

    pub fn current_map_id(&self) -> Option<MapID> {
        if let Some(tab) = self.tabs.get(self.current_tab) {
            match tab {
                AppTab::Map(maptab) => Some(maptab.id),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn current_map_ref(&self) -> Option<&CelesteMap> {
        if let Some(AppTab::Map(maptab)) = self.tabs.get(self.current_tab) {
            self.loaded_maps.get(&maptab.id).map(|s| &s.map)
        } else {
            None
        }
    }

    pub fn current_room_ref(&self) -> Option<&CelesteMapLevel> {
        if let Some(AppTab::Map(maptab)) = self.tabs.get(self.current_tab) {
            self.loaded_maps
                .get(&maptab.id)
                .and_then(|map| map.map.levels.get(maptab.current_room))
        } else {
            None
        }
    }

    pub fn garbage_collect(&mut self) {
        // destroy any tabs related to resources which no longer exist or are marked for closure
        // compute the new current-tab index
        let mut idx = 0;
        let mut current_delta: usize = 0;
        self.tabs.retain(|tab| {
            let closure = |idx: usize, tab: &AppTab| -> bool {
                if idx == self.poison_tab {
                    return false;
                }

                match tab {
                    AppTab::ProjectOverview(project) => self.modules.contains_key(project),
                    AppTab::Map(maptab) => self
                        .modules
                        .contains_key(&self.loaded_maps.get(&maptab.id).unwrap().path.module),
                    _ => true,
                }
            };

            let result = closure(idx, tab);
            if !result && self.current_tab >= idx {
                current_delta += 1;
            }
            idx += 1;
            result
        });
        self.current_tab = self.current_tab.saturating_sub(current_delta);
        self.poison_tab = usize::MAX;

        // collect a list of maps which need to be retained in memory based on open tabs
        let mut open_maps = HashSet::new();
        for tab in &self.tabs {
            #[allow(clippy::single_match)] // we will want more arms in the future
            match tab {
                AppTab::Map(maptab) => {
                    open_maps.insert(maptab.id);
                }
                _ => {}
            }
        }
        self.loaded_maps.retain(|id, _| open_maps.contains(id));
        self.loaded_maps_lookup
            .retain(|_, id| open_maps.contains(id));
    }

    pub fn map_action(&self, event: MapAction, merge_phase: EventPhase) -> AppEvent {
        AppEvent::MapEvent {
            map: Some(self.map_tab_unwrap().id),
            event: MapEvent::Action {
                merge_phase,
                event: RefCell::new(Some(event)),
            },
        }
    }

    pub fn map_action_unique(&self, event: MapAction) -> AppEvent {
        self.map_action(event, EventPhase::new())
    }

    pub fn room_action(&self, event: RoomAction, merge_phase: EventPhase) -> AppEvent {
        self.room_action_explicit(event, merge_phase, self.map_tab_unwrap().current_room)
    }

    pub fn room_action_explicit(
        &self,
        event: RoomAction,
        merge_phase: EventPhase,
        room: usize,
    ) -> AppEvent {
        self.map_action(MapAction::RoomAction { idx: room, event }, merge_phase)
    }

    // pub fn room_event_unique(&self, event: RoomAction) -> AppEvent {
    //     self.room_action(event, EventPhase::new())
    // }

    // pub fn room_event_unique_explicit(&self, event: RoomAction, room: usize) -> AppEvent {
    //     self.room_action_explicit(event, EventPhase::new(), room)
    // }

    pub fn batch_action(
        &self,
        events: impl IntoIterator<Item = MapAction>,
        merge_phase: EventPhase,
    ) -> AppEvent {
        self.map_action(
            MapAction::Batched {
                events: events.into_iter().collect(),
            },
            merge_phase,
        )
    }

    pub fn batch_action_unique(&self, events: impl IntoIterator<Item = MapAction>) -> AppEvent {
        self.batch_action(events, EventPhase::new())
    }
}

pub fn build_modules_lookup(
    modules: &HashMap<ModuleID, CelesteModule>,
) -> HashMap<String, ModuleID> {
    let mut result = HashMap::new();
    for (id, module) in modules.iter() {
        step_modules_lookup(&mut result, modules, *id, module);
    }
    result
}

pub fn step_modules_lookup(
    lookup: &mut HashMap<String, ModuleID>,
    modules: &HashMap<ModuleID, CelesteModule>,
    id: ModuleID,
    module: &CelesteModule,
) {
    match lookup.entry(module.everest_metadata.name.clone()) {
        Entry::Occupied(mut e) => {
            let path_existing = modules.get(e.get()).unwrap().filesystem_root.as_ref();
            let path_new = module.filesystem_root.as_ref();
            let ext_existing = path_existing
                .map(|root| root.extension().unwrap_or_else(|| OsStr::new("")))
                .and_then(|ext| ext.to_str());
            let ext_new = path_new
                .map(|root| root.extension().unwrap_or_else(|| OsStr::new("")))
                .and_then(|ext| ext.to_str());
            if ext_existing == Some("zip") && ext_new == Some("") {
                log::info!(
                    "Conflict between {} and {}, picked latter",
                    path_existing.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                    path_new.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                );
                e.insert(id);
            } else if ext_existing == Some("") && ext_new == Some("zip") {
                log::info!(
                    "Conflict between {} and {}, picked former",
                    path_existing.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                    path_new.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                );
            } else {
                log::warn!(
                    "Conflict between {} and {}, picked latter",
                    path_existing.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                    path_new.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                );
            }
        }
        Entry::Vacant(v) => {
            v.insert(id);
        }
    }
}
