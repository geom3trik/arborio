use arborio_modloader::module::{CelesteModuleKind, MapPath, ModuleID};
use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::project_map::ProjectEvent;
use arborio_state::lenses::StaticerLens;
use arborio_utils::vizia::prelude::*;
use arborio_widgets_common::common::label_with_pencil;

pub fn build_project_tab(cx: &mut Context, project: ModuleID) {
    ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
        VStack::new(cx, move |cx| {
            Binding::new(cx, AppState::modules_version, move |cx, _| {
                build_title(cx, project);
                build_map_list(cx, project);
                build_controls(cx, project);
            });
        })
        .id("maps_container");
    });
}

fn build_title(cx: &mut Context, project: ModuleID) {
    let module = cx
        .data::<AppState>()
        .unwrap()
        .modules
        .get(&project)
        .unwrap();
    let module_root = module.filesystem_root.clone();
    let module_name = module.everest_metadata.name.clone();
    let module_version = module.everest_metadata.version.clone();

    let editable = matches!(module.module_kind(), CelesteModuleKind::Directory);
    label_with_pencil(
        cx,
        StaticerLens::new(module_name),
        |_, _| true,
        move |cx, value| {
            cx.emit(AppEvent::ProjectEvent {
                project: Some(project),
                event: ProjectEvent::SetName { name: value },
            });
        },
        editable,
    )
    .class("project_name")
    .class("pencilable");
    HStack::new(cx, move |cx| {
        label_with_pencil(
            cx,
            StaticerLens::new(module_version),
            |_, _| true,
            move |cx, value| {
                cx.emit(AppEvent::ProjectEvent {
                    project: Some(project),
                    event: ProjectEvent::SetVersion { version: value },
                });
            },
            editable,
        )
        .class("project_version")
        .class("pencilable");
        Label::new(cx, " - ");
        label_with_pencil(
            cx,
            StaticerLens::new(
                module_root
                    .clone()
                    .map(|path| path.to_str().unwrap().to_owned())
                    .unwrap_or_else(|| "<built-in>".to_owned()),
            ),
            move |_, val| {
                val.starts_with(
                    module_root
                        .as_ref()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .to_str()
                        .unwrap(),
                )
            },
            move |cx, val| {
                cx.emit(AppEvent::ProjectEvent {
                    project: Some(project),
                    event: ProjectEvent::SetPath { path: val.into() },
                });
            },
            editable,
        )
        .class("project_path")
        .class("pencilable");
    });
}

fn build_map_list(cx: &mut Context, project: ModuleID) {
    let module = cx
        .data::<AppState>()
        .unwrap()
        .modules
        .get(&project)
        .unwrap();
    let mut maps = module.maps.to_vec();

    Label::new(cx, "Maps").class("module_category");
    HStack::new(cx, move |cx| {
        Label::new(cx, "+").class("big_plus");
        Label::new(cx, "New Map").id("new_map_text");
    })
    .class("btn_highlight")
    .id("new_map_button")
    .on_press(move |cx| {
        cx.emit(AppEvent::ProjectEvent {
            project: Some(project),
            event: ProjectEvent::NewMap,
        })
    });

    maps.sort();
    for map in maps.into_iter() {
        let map2 = map.clone();
        VStack::new(cx, move |cx| {
            Label::new(cx, &map2).class("map_title");
        })
        .class("map_overview_card")
        .class("btn_highlight")
        .on_press(move |cx| {
            cx.emit(AppEvent::OpenMap {
                path: MapPath {
                    module: project,
                    sid: map.clone(),
                },
            });
        });
    }
}

#[derive(Debug, Clone, Lens)]
struct DeleteState {
    started: bool,
    validated: bool,
}

#[derive(Debug)]
enum DeleteEvent {
    Start,
    Cancel,
    Validate(bool),
}

impl Model for DeleteState {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|msg, _| match msg {
            DeleteEvent::Start => self.started = true,
            DeleteEvent::Cancel => self.started = false,
            DeleteEvent::Validate(b) => self.validated = *b,
        });
    }
}

fn build_controls(cx: &mut Context, project: ModuleID) {
    let module = cx
        .data::<AppState>()
        .unwrap()
        .modules
        .get(&project)
        .unwrap();
    let module_name = module.everest_metadata.name.clone();
    let editing = matches!(module.module_kind(), CelesteModuleKind::Directory);
    VStack::new(cx, move |cx| {
        if editing {
            DeleteState {
                started: false,
                validated: false,
            }
            .build(cx);
            Binding::new(cx, DeleteState::started, move |cx, started| {
                let module_name = module_name.clone();
                if started.get(cx) {
                    VStack::new(cx, move |cx| {
                        Label::new(
                            cx,
                            "Type the name of the mod to continue. This cannot be undone!",
                        );
                        HStack::new(cx, move |cx| {
                            Textbox::new(cx, StaticerLens::new("")).on_edit(move |cx, value| {
                                cx.emit(DeleteEvent::Validate(value == module_name))
                            });
                            Label::new(cx, "Delete Project")
                                .class("btn_highlight")
                                .class("danger")
                                .id("delete_project_confirm")
                                .on_press(move |cx| {
                                    if DeleteState::validated.get(cx) {
                                        cx.emit(AppEvent::ProjectEvent {
                                            project: Some(project),
                                            event: ProjectEvent::Delete,
                                        });
                                    }
                                });
                            Label::new(cx, "Cancel")
                                .class("btn_highlight")
                                .id("delete_project_cancel")
                                .on_press(move |cx| {
                                    cx.emit(DeleteEvent::Cancel);
                                });
                        });
                    })
                    .id("delete_project_confirm_controls");
                } else {
                    Label::new(cx, "Delete Project")
                        .class("btn_highlight")
                        .class("danger")
                        .id("delete_project_start")
                        .on_press(move |cx| {
                            cx.emit(DeleteEvent::Start);
                        });
                }
            });
        }
    })
    .id("project_controls");
}
