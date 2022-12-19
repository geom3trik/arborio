use std::rc::Rc;

use arborio_maploader::map_struct::{Attribute, CelesteMapStyleground};
use arborio_modloader::config::AttributeType;
use arborio_state::data::action::{MapAction, StylegroundSelection};
use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::project_map::MapStateData;
use arborio_state::data::EventPhase;
use arborio_state::lenses::{
    CurrentMapImplLens, CurrentMapLens, CurrentStylegroundImplLens, CurrentStylegroundLens,
    IsFailedLens, StylegroundNameLens,
};
use arborio_utils::vizia::fonts::icons_names::DOWN;
use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::state::UnwrapLens;
use arborio_widgets_common::advanced_tweaker::*;

macro_rules! edit_text {
    ($cx: expr, $label:expr, $attr:ident) => {
        tweak_attr_text(
            $cx,
            $label,
            CurrentStylegroundImplLens {}.then(CelesteMapStyleground::$attr),
            |cx, x| {
                let mut style = CurrentStylegroundImplLens {}.get(cx);
                style.$attr = x;
                emit(cx, style);
                true
            },
        );
    };
}
macro_rules! edit_check {
    ($cx: expr, $label:expr, $attr:ident) => {
        tweak_attr_check(
            $cx,
            $label,
            CurrentStylegroundImplLens {}.then(CelesteMapStyleground::$attr),
            |cx, x| {
                let mut style = CurrentStylegroundImplLens {}.get(cx);
                style.$attr = x;
                emit(cx, style);
            },
        );
    };
}
macro_rules! edit_optional_text {
    ($cx: expr, $label:expr, $attr:ident) => {
        tweak_attr_text(
            $cx,
            $label,
            CurrentStylegroundImplLens {}
                .then(CelesteMapStyleground::$attr)
                .then(UnwrapLens::new()),
            |cx, x| {
                let mut style = CurrentStylegroundImplLens {}.get(cx);
                style.$attr = if x.is_empty() { None } else { Some(x) };
                emit(cx, style);
                true
            },
        );
    };
}

pub struct StyleListWidget {}

impl StyleListWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}.build(cx, move |cx| {
            ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                Label::new(cx, "Foregrounds").class("style_category");
                build_active_style_list(
                    cx,
                    true,
                    CurrentMapImplLens {}.then(MapStateData::foregrounds),
                );
                Label::new(cx, "Backgrounds").class("style_category");
                build_active_style_list(
                    cx,
                    false,
                    CurrentMapImplLens {}.then(MapStateData::backgrounds),
                );
            });
        })
    }
}

impl View for StyleListWidget {
    fn element(&self) -> Option<&'static str> {
        Some("style_list")
    }
}

fn build_active_style_list<L>(cx: &mut Context, fg: bool, lens: L)
where
    L: Lens<Target = Vec<CelesteMapStyleground>> + Copy,
    <L as Lens>::Source: Model,
{
    Binding::new(cx, lens.map(|vec| vec.len()), move |cx, len_lens| {
        for idx in (0..len_lens.get_fallible(cx).unwrap_or(0)).rev() {
            let lens = lens.index(idx);
            HStack::new(cx, move |cx| {
                Label::new(cx, lens.then(StylegroundNameLens {}));
            })
            .class("palette_item")
            .class("list_highlight")
            .bind(CurrentStylegroundLens {}, move |handle, selected| {
                let is_me =
                    selected.get_fallible(handle.cx) == Some(StylegroundSelection { fg, idx });
                handle.checked(is_me);
            })
            .on_press(move |cx| {
                let tab = cx.data::<AppState>().unwrap().current_tab;
                cx.emit(AppEvent::SelectStyleground {
                    tab,
                    styleground: Some(StylegroundSelection { fg, idx }),
                });
            });
        }
    });
}

pub struct StyleTweakerWidget {}

impl StyleTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}
            .build(cx, |cx| {
                HStack::new(cx, |cx| {
                    Button::new(
                        cx,
                        |cx| {
                            if (CurrentStylegroundLens {}).get_fallible(cx).is_some() {
                                cx.emit(CurrentMapLens {}.get(cx).action(
                                    EventPhase::new(),
                                    MapAction::AddStyleground {
                                        loc: CurrentStylegroundLens {}.get(cx),
                                        style: Box::<
                                            arborio_maploader::map_struct::CelesteMapStyleground,
                                        >::default(),
                                    },
                                ));
                            }
                        },
                        |cx| Label::new(cx, "\u{e145}").class("icon"),
                    );
                    Button::new(
                        cx,
                        |cx| {
                            if (CurrentStylegroundLens {}).get_fallible(cx).is_some() {
                                cx.emit(CurrentMapLens {}.get(cx).action(
                                    EventPhase::new(),
                                    MapAction::RemoveStyleground {
                                        loc: CurrentStylegroundLens {}.get(cx),
                                    },
                                ));
                            }
                        },
                        |cx| Label::new(cx, "\u{e15b}").class("icon"),
                    );
                    Button::new(
                        cx,
                        |cx| {
                            let sel =
                                if let Some(sel) = (CurrentStylegroundLens {}).get_fallible(cx) {
                                    sel
                                } else {
                                    return;
                                };
                            let max_idx = CurrentMapImplLens {}
                                .map(move |map| map.styles(sel.fg).len())
                                .get(cx);
                            let target = if sel.idx + 1 == max_idx {
                                if sel.fg {
                                    return;
                                }
                                StylegroundSelection { fg: true, idx: 0 }
                            } else {
                                StylegroundSelection {
                                    fg: sel.fg,
                                    idx: sel.idx + 1,
                                }
                            };
                            cx.emit(CurrentMapLens {}.get(cx).action(
                                EventPhase::new(),
                                MapAction::MoveStyleground { loc: sel, target },
                            ));
                            cx.emit(AppEvent::SelectStyleground {
                                tab: cx.data::<AppState>().unwrap().current_tab,
                                styleground: Some(target),
                            })
                        },
                        |cx| Label::new(cx, "\u{e5ce}").class("icon"),
                    );
                    Button::new(
                        cx,
                        |cx| {
                            let sel =
                                if let Some(sel) = (CurrentStylegroundLens {}).get_fallible(cx) {
                                    sel
                                } else {
                                    return;
                                };
                            let target = if sel.idx == 0 {
                                if !sel.fg {
                                    return;
                                }
                                let max_idx = CurrentMapImplLens {}
                                    .map(move |map| map.styles(false).len())
                                    .get(cx);
                                StylegroundSelection {
                                    fg: false,
                                    idx: max_idx,
                                }
                            } else {
                                StylegroundSelection {
                                    fg: sel.fg,
                                    idx: sel.idx - 1,
                                }
                            };
                            cx.emit(CurrentMapLens {}.get(cx).action(
                                EventPhase::new(),
                                MapAction::MoveStyleground { loc: sel, target },
                            ));
                            cx.emit(AppEvent::SelectStyleground {
                                tab: cx.data::<AppState>().unwrap().current_tab,
                                styleground: Some(target),
                            })
                        },
                        |cx| Label::new(cx, "\u{e5cf}").class("icon"),
                    );
                });
                ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                    Binding::new(
                        cx,
                        IsFailedLens::new(CurrentStylegroundImplLens {}),
                        move |cx, is_failed| {
                            if !is_failed.get(cx) {
                                Self::members(cx);
                            }
                        },
                    );
                });
            })
            .class("tweaker")
    }

    fn members(cx: &mut Context) {
        edit_text!(cx, "Name", name);
        edit_text!(cx, "Tag", tag);
        edit_text!(cx, "X", x);
        edit_text!(cx, "Y", y);
        edit_text!(cx, "Scroll X", scroll_x);
        edit_text!(cx, "Scroll Y", scroll_y);
        edit_text!(cx, "Speed X", speed_x);
        edit_text!(cx, "Speed Y", speed_y);
        edit_text!(cx, "Color", color); // TODO real validation
        edit_text!(cx, "Alpha", alpha);
        edit_check!(cx, "Flip X", flip_x);
        edit_check!(cx, "Flip Y", flip_y);
        edit_check!(cx, "Loop X", loop_x);
        edit_check!(cx, "Loop Y", loop_y);
        edit_text!(cx, "Wind", wind);
        edit_check!(cx, "Instant In", instant_in);
        edit_check!(cx, "Instant out", instant_out);
        edit_optional_text!(cx, "Show If Flag", flag);
        edit_optional_text!(cx, "Hide If Flag", not_flag);
        edit_optional_text!(cx, "Override If Flag", always);
        tweak_attr_picker(
            cx,
            "Dreaming Status",
            CurrentStylegroundImplLens {}.then(CelesteMapStyleground::dreaming),
            [None, Some(true), Some(false)],
            |_, item| {
                match item {
                    // clion has a false-positive error here
                    None => "Both",
                    Some(true) => "Dreaming",
                    Some(false) => "Awake",
                }
                .to_owned()
            },
            |cx, item| {
                let mut style = CurrentStylegroundImplLens {}.get(cx);
                style.dreaming = item;
                emit(cx, style);
            },
        );
        edit_optional_text!(cx, "Exclude Rooms", exclude);
        edit_optional_text!(cx, "Only Rooms", only);
        edit_text!(cx, "Fade X", fade_x);
        edit_text!(cx, "Fade Y", fade_y);

        advanced_attrs_editor(
            cx,
            CurrentStylegroundImplLens {}.then(CelesteMapStyleground::attributes),
            |cx, key, value| {
                let mut current = CurrentStylegroundImplLens {}.get(cx);
                current.attributes.insert(key, value);
                emit(cx, current);
            },
            |cx, key, ty| {
                let mut current = CurrentStylegroundImplLens {}.get(cx);
                current.attributes.insert(
                    key,
                    match ty {
                        AttributeType::String => Attribute::Text("".to_owned()),
                        AttributeType::Float => Attribute::Float(0.0),
                        AttributeType::Int => Attribute::Int(0),
                        AttributeType::Bool => Attribute::Bool(false),
                    },
                );
                emit(cx, current);
            },
            |cx, key| {
                let mut current = CurrentStylegroundImplLens {}.get(cx);
                current.attributes.remove(&key);
                emit(cx, current);
            },
        );
    }
}

impl View for StyleTweakerWidget {
    fn element(&self) -> Option<&'static str> {
        Some("style_tweaker")
    }
}

fn emit(cx: &mut EventContext, style: CelesteMapStyleground) {
    cx.emit(CurrentMapLens {}.get(cx).action(
        EventPhase::new(),
        MapAction::UpdateStyleground {
            loc: CurrentStylegroundLens {}.get(cx),
            style: Box::new(style),
        },
    ));
}

fn tweak_attr_picker<T: Data>(
    // TODO move to common when mature
    cx: &mut Context,
    name: &'static str,
    lens: impl Lens<Target = T>,
    items: impl 'static + IntoIterator<Item = T> + Clone,
    labels: impl 'static + Fn(&mut Context, &T) -> String,
    setter: impl 'static + Fn(&mut EventContext, T),
) {
    let labels = Rc::new(labels);
    let labels2 = labels.clone();
    let setter = Rc::new(setter);
    HStack::new(cx, move |cx| {
        Label::new(cx, name);
        Dropdown::new(
            cx,
            move |cx| {
                let labels2 = labels2.clone();
                let lens = lens.clone();
                HStack::new(cx, move |cx| {
                    let labels2 = labels2.clone();
                    Label::new(cx, "").bind(lens, move |handle, item| {
                        if let Some(item) = item.get_fallible(handle.cx) {
                            let label = (labels2)(handle.cx, &item);
                            handle.text(&label);
                        }
                    });
                    Label::new(cx, DOWN).class("icon").class("dropdown_icon");
                })
            },
            move |cx| {
                let items = items.clone();
                for item in items.into_iter() {
                    let setter = setter.clone();
                    let label = labels(cx, &item);
                    Label::new(cx, &label)
                        .class("dropdown_element")
                        .class("btn_highlight")
                        .on_press(move |cx| {
                            cx.emit(PopupEvent::Close);
                            setter(cx, item.clone());
                        });
                }
            },
        );
    });
}