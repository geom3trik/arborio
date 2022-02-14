use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time;
use vizia::*;

use crate::assets;
use crate::auto_saver::AutoSaver;
use crate::celeste_mod::aggregate::ModuleAggregate;
use crate::celeste_mod::discovery;
use crate::celeste_mod::module::CelesteModule;
use crate::map_struct::{CelesteMap, CelesteMapDecal, CelesteMapEntity, CelesteMapLevel, MapID};
use crate::units::*;
use crate::widgets::palette_widget::{
    DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable,
};

#[derive(Lens)]
pub struct AppState {
    pub config: AutoSaver<AppConfig>,

    pub modules: HashMap<String, CelesteModule>,
    pub modules_version: u32,
    pub palettes: HashMap<String, ModuleAggregate>,
    pub loaded_maps: HashMap<MapID, CelesteMap>,

    pub current_tab: usize,
    pub tabs: Vec<AppTab>,

    pub current_tool: usize,
    pub current_layer: Layer,
    pub current_fg_tile: TileSelectable,
    pub current_bg_tile: TileSelectable,
    pub current_entity: EntitySelectable,
    pub current_trigger: TriggerSelectable,
    pub current_decal: DecalSelectable,
    pub current_selected: Option<AppSelection>, // awkward. should be part of editor state
    pub current_objtile: u32,
    pub objtiles_transform: MapToScreen,

    pub draw_interval: f32,
    pub snap: bool,

    pub last_draw: RefCell<time::Instant>, // mutable to draw
    pub progress: Progress,
}

#[derive(Serialize, Deserialize, Default, Lens, Debug)]
pub struct AppConfig {
    pub celeste_root: Option<PathBuf>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AppTab {
    CelesteOverview,
    ProjectOverview(String),
    Map(MapTab),
}

#[derive(Clone, Debug)]
pub struct MapTab {
    pub id: MapID,
    pub nonce: u32,
    pub current_room: usize,
    pub transform: MapToScreen,
}

impl PartialEq for MapTab {
    fn eq(&self, other: &Self) -> bool {
        self.nonce == other.nonce
    }
}

impl Eq for MapTab {}

impl Data for AppTab {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl ToString for AppTab {
    fn to_string(&self) -> String {
        match self {
            AppTab::CelesteOverview => "All Mods".to_owned(),
            AppTab::ProjectOverview(s) => format!("{} - Overview", s),
            AppTab::Map(m) => m.id.sid.clone(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, enum_iterator::IntoEnumIterator)]
pub enum Layer {
    FgTiles,
    BgTiles,
    FgDecals,
    BgDecals,
    Entities,
    Triggers,
    ObjectTiles,
    All,
}

impl Data for Layer {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl Layer {
    pub fn name(&self) -> &'static str {
        match self {
            Layer::FgTiles => "Foreground Tiles",
            Layer::BgTiles => "Background Tiles",
            Layer::Entities => "Entities",
            Layer::Triggers => "Triggers",
            Layer::FgDecals => "Foreground Decals",
            Layer::BgDecals => "Background Decals",
            Layer::ObjectTiles => "Object Tiles",
            Layer::All => "All Layers",
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub enum AppSelection {
    FgTile(TilePoint),
    BgTile(TilePoint),
    ObjectTile(TilePoint),
    EntityBody(i32, bool),
    EntityNode(i32, usize, bool),
    Decal(u32, bool),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Progress {
    pub progress: i32,
    pub status: String,
}

impl Data for Progress {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Debug)]
pub enum AppEvent {
    Progress {
        progress: Progress,
    },
    SetConfigPath {
        path: PathBuf,
    },
    SetModules {
        modules: Mutex<HashMap<String, CelesteModule>>,
    },
    OpenModuleOverview {
        module: String,
    },
    Load {
        map: RefCell<Option<Box<CelesteMap>>>,
    },
    SelectTab {
        idx: usize,
    },
    CloseTab {
        idx: usize,
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
        idx: usize,
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
        // TODO uhhhhhhhhhhhhhhhh
        selection: Option<AppSelection>,
    },
    TileUpdate {
        map: MapID,
        room: usize,
        fg: bool,
        offset: TilePoint,
        data: TileGrid<char>,
    },
    ObjectTileUpdate {
        map: MapID,
        room: usize,
        offset: TilePoint,
        data: TileGrid<i32>,
    },
    EntityAdd {
        map: MapID,
        room: usize,
        entity: CelesteMapEntity,
        trigger: bool,
    },
    EntityUpdate {
        map: MapID,
        room: usize,
        entity: CelesteMapEntity,
        trigger: bool,
    },
    EntityRemove {
        map: MapID,
        room: usize,
        id: i32,
        trigger: bool,
    },
    DecalAdd {
        map: MapID,
        room: usize,
        fg: bool,
        decal: CelesteMapDecal,
    },
    DecalUpdate {
        map: MapID,
        room: usize,
        fg: bool,
        decal: CelesteMapDecal,
    },
    DecalRemove {
        map: MapID,
        room: usize,
        fg: bool,
        id: u32,
    },
}

impl Model for AppState {
    fn event(&mut self, cx: &mut Context, event: &mut Event) {
        if let Some(app_event) = event.message.downcast() {
            self.apply(cx, app_event);
        }
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
                .unwrap_or_else(|e| panic!("Failed to save celeste_mod file: {}", e));
        });

        AppState {
            config: cfg,
            current_tab: 0,
            tabs: vec![AppTab::CelesteOverview],
            loaded_maps: HashMap::new(),
            current_tool: 2,
            current_fg_tile: TileSelectable::default(),
            current_bg_tile: TileSelectable::default(),
            current_entity: EntitySelectable::default(),
            current_trigger: TriggerSelectable::default(),
            current_decal: DecalSelectable::default(),
            current_selected: None,
            draw_interval: 4.0,
            snap: true,
            last_draw: RefCell::new(time::Instant::now()),
            current_layer: Layer::FgTiles,
            current_objtile: 0,
            objtiles_transform: MapToScreen::identity(),

            modules: HashMap::new(),
            modules_version: 0,
            palettes: HashMap::new(),
            progress: Progress {
                progress: 100,
                status: "".to_owned(),
            },
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

    pub fn current_palette_unwrap(&self) -> &ModuleAggregate {
        if let Some(AppTab::Map(result)) = self.tabs.get(self.current_tab) {
            self.palettes
                .get(&result.id.module)
                .expect("stale reference")
        } else {
            panic!("misuse of current_palette_unwrap");
        }
    }

    pub fn current_room_ref(&self) -> Option<&CelesteMapLevel> {
        if let Some(AppTab::Map(maptab)) = self.tabs.get(self.current_tab) {
            self.loaded_maps
                .get(&maptab.id)
                .and_then(|map| map.levels.get(maptab.current_room))
        } else {
            None
        }
    }

    pub fn apply(&mut self, cx: &mut Context, event: &AppEvent) {
        match event {
            // global events
            AppEvent::Progress { progress } => {
                self.progress = progress.clone();
            }
            AppEvent::SelectObject { selection } => {
                self.current_selected = *selection;
                if let Some(room) = self.current_room_ref() {
                    room.cache.borrow_mut().render_cache_valid = false;
                }
            }
            AppEvent::OpenModuleOverview { module } => {
                for (i, tab) in self.tabs.iter().enumerate() {
                    if matches!(tab, AppTab::ProjectOverview(m) if m == module) {
                        cx.emit(AppEvent::SelectTab { idx: i });
                        return;
                    }
                }
                self.tabs.push(AppTab::ProjectOverview(module.clone()));
                cx.emit(AppEvent::SelectTab {
                    idx: self.tabs.len() - 1,
                });
            }
            AppEvent::Load { map } => {
                if let Some(map) = map.borrow_mut().take() {
                    if !self.loaded_maps.contains_key(&map.id) {
                        self.current_tab = self.tabs.len();
                        self.tabs.push(AppTab::Map(MapTab {
                            nonce: assets::next_uuid(),
                            id: map.id.clone(),
                            current_room: 0,
                            transform: MapToScreen::identity(),
                        }));
                    }

                    if !self.palettes.contains_key(&map.id.module) {
                        self.palettes.insert(
                            map.id.module.clone(),
                            ModuleAggregate::new(&self.modules, &map.id.module),
                        );
                    }

                    self.loaded_maps.insert(map.id.clone(), *map);
                }
            }
            AppEvent::SetConfigPath { path } => {
                self.config.borrow_mut().celeste_root = Some(path.clone());
                trigger_module_load(cx, path.clone());
            }
            AppEvent::SetModules { modules } => {
                let mut r = modules.lock().unwrap();
                std::mem::swap(r.deref_mut(), &mut self.modules);
                self.modules_version += 1;
                trigger_palette_update(&mut self.palettes, &self.modules);
            }
            AppEvent::SelectTool { idx } => {
                self.current_tool = *idx;
            }
            AppEvent::SelectLayer { layer } => {
                self.current_layer = *layer;
            }
            AppEvent::SelectPaletteTile { fg, tile } => {
                if *fg {
                    self.current_fg_tile = *tile;
                } else {
                    self.current_bg_tile = *tile;
                }
            }
            AppEvent::SelectPaletteObjectTile { tile } => {
                self.current_objtile = *tile;
            }
            AppEvent::SelectPaletteEntity { entity } => {
                self.current_entity = *entity;
            }
            AppEvent::SelectPaletteTrigger { trigger } => {
                self.current_trigger = *trigger;
            }
            AppEvent::SelectPaletteDecal { decal } => {
                self.current_decal = *decal;
            }
            AppEvent::PanObjectTiles { delta } => {
                // TODO limits
                self.objtiles_transform = self.objtiles_transform.pre_translate(*delta);
            }
            AppEvent::ZoomObjectTiles { delta, focus } => {
                self.objtiles_transform = self
                    .objtiles_transform
                    .pre_translate(focus.to_vector())
                    .pre_scale(*delta, *delta)
                    .pre_translate(-focus.to_vector());
            }

            // tab events
            AppEvent::SelectTab { idx } => {
                if *idx < self.tabs.len() {
                    self.current_tab = *idx;
                }
            }
            AppEvent::CloseTab { idx } => {
                self.tabs.remove(*idx);
                if (self.current_tab > *idx || self.current_tab >= self.tabs.len())
                    && self.current_tab > 0
                {
                    self.current_tab -= 1;
                }
                self.garbage_collect();
            }
            AppEvent::Pan { tab, delta } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(*tab) {
                    map_tab.transform = map_tab.transform.pre_translate(*delta);
                }
            }
            AppEvent::Zoom { tab, delta, focus } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(*tab) {
                    // TODO scale stepping, high and low limits
                    map_tab.transform = map_tab
                        .transform
                        .pre_translate(focus.to_vector())
                        .pre_scale(*delta, *delta)
                        .pre_translate(-focus.to_vector());
                }
            }
            AppEvent::SelectRoom { tab, idx } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(*tab) {
                    map_tab.current_room = *idx;
                    if let Some(room) = self.current_room_ref() {
                        room.cache.borrow_mut().render_cache_valid = false;
                    }
                }
            }

            // room events
            AppEvent::TileUpdate {
                map,
                room,
                fg,
                offset,
                data,
            } => {
                if let Some(map) = self.loaded_maps.get_mut(map) {
                    if let Some(room) = map.levels.get_mut(*room) {
                        let target = if *fg {
                            &mut room.fg_tiles
                        } else {
                            &mut room.bg_tiles
                        };
                        let dirty = apply_tiles(offset, data, target, '\0');
                        if dirty {
                            room.cache.borrow_mut().render_cache_valid = false;
                            map.dirty = true;
                        }
                    }
                }
            }
            AppEvent::ObjectTileUpdate {
                map,
                room,
                offset,
                data,
            } => {
                if let Some(map) = self.loaded_maps.get_mut(map) {
                    if let Some(room) = map.levels.get_mut(*room) {
                        let dirty = apply_tiles(offset, data, &mut room.object_tiles, -2);
                        if dirty {
                            room.cache.borrow_mut().render_cache_valid = false;
                            map.dirty = true;
                        }
                    }
                }
            }
            AppEvent::EntityAdd {
                map,
                room,
                entity,
                trigger,
            } => {
                if let Some(room) = self
                    .loaded_maps
                    .get_mut(map)
                    .and_then(|map| map.levels.get_mut(*room))
                {
                    let mut entity = entity.clone();
                    entity.id = room.next_id();
                    if *trigger {
                        room.triggers.push(entity);
                    } else {
                        room.entities.push(entity)
                    }
                    room.cache.borrow_mut().render_cache_valid = false;
                    self.loaded_maps.get_mut(map).unwrap().dirty = true;
                }
            }
            AppEvent::EntityUpdate {
                map,
                room,
                entity,
                trigger,
            } => {
                if let Some(room) = self
                    .loaded_maps
                    .get_mut(map)
                    .and_then(|map| map.levels.get_mut(*room))
                {
                    if let Some(e) = room.entity_mut(entity.id, *trigger) {
                        *e = entity.clone();
                        room.cache.borrow_mut().render_cache_valid = false;
                        self.loaded_maps.get_mut(map).unwrap().dirty = true;
                    }
                }
            }
            AppEvent::EntityRemove {
                map,
                room,
                id,
                trigger,
            } => {
                if let Some(room) = self
                    .loaded_maps
                    .get_mut(map)
                    .and_then(|map| map.levels.get_mut(*room))
                {
                    // tfw drain_filter is unstable
                    let mut i = 0;
                    let mut any = false;
                    let entities = if *trigger {
                        &mut room.triggers
                    } else {
                        &mut room.entities
                    };
                    while i < entities.len() {
                        if entities[i].id == *id {
                            entities.remove(i);
                            any = true;
                        } else {
                            i += 1;
                        }
                    }
                    if any {
                        room.cache.borrow_mut().render_cache_valid = false;
                        self.loaded_maps.get_mut(map).unwrap().dirty = true;
                    }
                }
            }
            AppEvent::DecalAdd {
                map,
                room,
                fg,
                decal,
            } => {
                if let Some(room) = self
                    .loaded_maps
                    .get_mut(map)
                    .and_then(|map| map.levels.get_mut(*room))
                {
                    let mut decal = decal.clone();
                    let decals = if *fg {
                        &mut room.fg_decals
                    } else {
                        &mut room.bg_decals
                    };
                    decal.id = assets::next_uuid();
                    decals.push(decal);
                    room.cache.borrow_mut().render_cache_valid = false;
                    self.loaded_maps.get_mut(map).unwrap().dirty = true;
                }
            }
            AppEvent::DecalUpdate {
                map,
                room,
                fg,
                decal,
            } => {
                if let Some(room) = self
                    .loaded_maps
                    .get_mut(map)
                    .and_then(|map| map.levels.get_mut(*room))
                {
                    if let Some(decal_dest) = room.decal_mut(decal.id, *fg) {
                        *decal_dest = decal.clone();
                        room.cache.borrow_mut().render_cache_valid = false;
                        self.loaded_maps.get_mut(map).unwrap().dirty = true;
                    }
                }
            }
            AppEvent::DecalRemove { map, room, fg, id } => {
                if let Some(room) = self
                    .loaded_maps
                    .get_mut(map)
                    .and_then(|map| map.levels.get_mut(*room))
                {
                    // tfw drain_filter is unstable
                    let mut i = 0;
                    let mut any = false;
                    let decals = if *fg {
                        &mut room.fg_decals
                    } else {
                        &mut room.bg_decals
                    };
                    while i < decals.len() {
                        if decals[i].id == *id {
                            decals.remove(i);
                            any = true;
                        } else {
                            i += 1;
                        }
                    }
                    if any {
                        room.cache.borrow_mut().render_cache_valid = false;
                        self.loaded_maps.get_mut(map).unwrap().dirty = true;
                    }
                }
            }
        }
    }

    pub fn garbage_collect(&mut self) {
        let mut open_maps = HashSet::new();
        for tab in &self.tabs {
            #[allow(clippy::single_match)] // we will want more arms in the future
            match tab {
                AppTab::Map(maptab) => {
                    open_maps.insert(maptab.id.clone());
                }
                _ => {}
            }
        }
        let open_palettes = open_maps
            .iter()
            .map(|id| &id.module)
            .collect::<HashSet<_>>();
        self.loaded_maps.retain(|id, _| open_maps.contains(id));
        self.palettes.retain(|name, _| open_palettes.contains(name));
    }
}

pub fn apply_tiles<T: Copy + Eq>(
    offset: &TilePoint,
    data: &TileGrid<T>,
    target: &mut TileGrid<T>,
    ignore: T,
) -> bool {
    let mut dirty = false;
    let mut line_start = *offset;
    let mut cur = line_start;
    for (idx, tile) in data.tiles.iter().enumerate() {
        if *tile != ignore {
            if let Some(tile_ref) = target.get_mut(cur) {
                if *tile_ref != *tile {
                    *tile_ref = *tile;
                    dirty = true;
                }
            }
        }
        if (idx + 1) % data.stride == 0 {
            line_start += TileVector::new(0, 1);
            cur = line_start;
        } else {
            cur += TileVector::new(1, 0);
        }
    }
    dirty
}

pub fn trigger_module_load(cx: &mut Context, path: PathBuf) {
    cx.spawn(move |cx| {
        let mut result = HashMap::new();
        discovery::load_all(&path, &mut result, |p, s| {
            cx.emit(AppEvent::Progress {
                progress: Progress {
                    progress: (p * 100.0) as i32,
                    status: s,
                },
            })
            .unwrap();
        });
        cx.emit(AppEvent::Progress {
            progress: Progress {
                progress: 100,
                status: "".to_owned(),
            },
        })
        .unwrap();
        cx.emit(AppEvent::SetModules {
            modules: Mutex::new(result),
        })
        .unwrap();
    })
}

pub fn trigger_palette_update(
    palettes: &mut HashMap<String, ModuleAggregate>,
    modules: &HashMap<String, CelesteModule>,
) {
    for (name, pal) in palettes {
        *pal = ModuleAggregate::new(modules, name);
    }
}
