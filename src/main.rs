#![allow(unused)]

mod editor_widget;
mod map_struct;
mod atlas_img;
mod autotiler;
mod assets;
mod auto_saver;
mod entity_config;
mod entity_expression;
mod app_state;
mod tools;
mod units;

use std::fs;
use std::cell::RefCell;
use std::error::Error;
use vizia::*;
use dialog::{DialogBox, FileSelectionMode};
use crate::app_state::AppEvent;

fn main() -> Result<(), Box<dyn Error>> {
    assets::load();

    let mut app = Application::new(
        WindowDescription::new()
            .with_title("Arborio"),
        |cx| {
            app_state::AppState::new().build(cx);

            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    Button::new(
                        cx,
                        |cx| {
                            let path = match dialog::FileSelection::new("Select a map")
                                .title("Select a map")
                                .mode(FileSelectionMode::Open)
                                .path(assets::CONFIG.lock().unwrap().celeste_root.to_path_buf())
                                .show() {
                                Ok(Some(path)) => path,
                                _ => return
                            };
                            let file = match std::fs::read(path) {
                                Ok(data) => data,
                                Err(e) => {
                                    dialog::Message::new(format!("Could not read file: {}", e)).show();
                                    return
                                }
                            };
                            let (_, binfile) = match celeste::binel::parser::take_file(file.as_slice()) {
                                Ok(binel) => binel,
                                _ => {
                                    dialog::Message::new("Not a Celeste map").show();
                                    return
                                }
                            };
                            let map = match map_struct::from_binfile(binfile) {
                                Ok(map) => map,
                                Err(e) => {
                                    dialog::Message::new(format!("Data validation error: {}", e));
                                    return
                                }
                            };
                            cx.emit(AppEvent::Load { map: RefCell::new(Some(map)) });
                        },
                        |cx| Label::new(cx, "Load Map")
                    );
                })
                    .height(Pixels(30.0));;
                let _ed = editor_widget::EditorWidget::new(cx)
                    .width(Stretch(1.0))
                    .height(Stretch(1.0));
                dbg!("editor is", _ed.entity);
            });
        });

    app.run();
    Ok(())
}

