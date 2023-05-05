use arborio_modloader::aggregate::ModuleAggregate;
use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::{AppConfig, AppConfigSetter, Layer};
use arborio_state::lenses::{current_map_lens, current_palette_lens, AnotherLens, AutoSaverLens};
use arborio_state::tools::ToolSpec;
use arborio_utils::vizia::prelude::*;

use arborio_widgets_common::list_palette::PaletteWidget;
use arborio_widgets_editor::editor::EditorWidget;
use arborio_widgets_editor_entity::entity_tweaker::EntityTweakerWidget;
use arborio_widgets_editor_room::room_tweaker::RoomTweakerWidget;
use arborio_widgets_editor_style::style_tweaker::{StyleListWidget, StyleTweakerWidget};
use arborio_widgets_tilepicker::tile_palette::TilePaletteWidget;

pub fn build_editor(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            build_tool_picker(cx);
        })
        .id("left_bar");

        VStack::new(cx, move |cx| {
            HStack::new(cx, move |cx| {
                build_tool_settings(cx);
            })
            .id("tool_settings");
            EditorWidget::new(cx)
                .width(Stretch(1.0))
                .height(Stretch(1.0));
        }).height(Stretch(1.0));

        VStack::new(cx, |cx| {
            build_layer_picker(cx);
            build_palette_widgets(cx);
            build_tweaker_widgets(cx);
        })
        .id("right_bar");
    })
    .width(Stretch(1.0));
}

fn build_tool_settings(cx: &mut Context) {
    HStack::new(cx, move |cx| {
        Label::new(cx, "Snap").describing("tool_settings_snap");
        let lens = AppState::config
            .then(AutoSaverLens::new())
            .then(AppConfig::snap);
        Checkbox::new(cx, lens)
            .on_toggle(move |cx| {
                let val = !lens.get(cx);
                cx.emit(AppEvent::EditSettings {
                    setter: AppConfigSetter::Snap(val),
                });
            })
            .id("tool_settings_snap");
    })
    .bind(AppState::current_toolspec, move |handle, spec| {
        let spec = spec.get(&handle);
        let show = spec == ToolSpec::Pencil || spec == ToolSpec::Selection;
        handle.display(show);
    });

    HStack::new(cx, move |cx| {
        Label::new(cx, "Interval").describing("tool_settings_interval");
        let lens = AppState::config
            .then(AutoSaverLens::new())
            .then(AppConfig::draw_interval);
        Slider::new(cx, lens)
            .range(1.0..100.0)
            .on_changing(|cx, val| {
                cx.emit(AppEvent::EditSettings {
                    setter: AppConfigSetter::DrawInterval(val),
                });
            })
            .id("tool_settings_interval");
    })
    .bind(AppState::current_toolspec, move |handle, spec| {
        let spec = spec.get(&handle);
        let show = spec == ToolSpec::Pencil;
        handle.display(show);
    });

    HStack::new(cx, move |cx| {
        Label::new(cx, "Advanced Tweaker").describing("tool_settings_advanced");
        let lens = AppState::config
            .then(AutoSaverLens::new())
            .then(AppConfig::advanced);
        Checkbox::new(cx, lens)
            .on_toggle(move |cx| {
                let val = !lens.get(cx);
                cx.emit(AppEvent::EditSettings {
                    setter: AppConfigSetter::Advanced(val),
                });
            })
            .id("tool_settings_advanced");
    })
    .bind(AppState::current_toolspec, move |handle, spec| {
        let spec = spec.get(&handle);
        let show = spec == ToolSpec::Selection;
        handle.display(show);
    });
}

pub fn build_tool_picker(cx: &mut Context) {
    VStack::new(cx, move |cx| {
        Binding::new(cx, current_map_lens(), |cx, map| {
            let showme = map.get_fallible(cx).is_some();
            if showme {
                Binding::new(cx, AppState::current_toolspec, |cx, tool_field| {
                    for toolspec in enum_iterator::all::<ToolSpec>() {
                        let selected = tool_field.map(move |sel| sel == &toolspec);
                        let selected2 = selected.clone();
                        HStack::new(cx, move |cx| {
                            RadioButton::new(cx, selected2.clone());
                            Label::new(cx, toolspec.name());
                        })
                        .on_press(move |cx| cx.emit(AppEvent::SelectTool { spec: toolspec }))
                        .checked(selected)
                        .class("list_highlight");
                    }
                })
            }
        });
    })
    .id("tool_picker");
}

pub fn build_layer_picker(cx: &mut Context) {
    VStack::new(cx, move |cx| {
        for layer in enum_iterator::all::<Layer>() {
            let selected = AppState::current_layer.map(move |sel| sel == &layer);
            let selected2 = selected.clone();
            HStack::new(cx, move |cx| {
                RadioButton::new(cx, selected2.clone());
                Label::new(cx, layer.name());
            })
            .on_press(move |cx| cx.emit(AppEvent::SelectLayer { layer }))
            .checked(selected)
            .class("list_highlight")
            .bind(AppState::current_toolspec, move |handle, toolspec| {
                let toolspec = toolspec.get(&handle);
                handle.display(
                    toolspec == ToolSpec::Selection
                        || (toolspec == ToolSpec::Bucket
                            && (layer == Layer::FgTiles || layer == Layer::BgTiles))
                        || (toolspec != ToolSpec::Bucket
                            && toolspec != ToolSpec::Selection
                            && layer != Layer::All),
                );
            });
        }
    })
    .id("layer_picker")
    .bind(AppState::current_toolspec, move |handle, toolspec| {
        let toolspec = toolspec.get(&handle);
        handle.display(toolspec != ToolSpec::Style);
    });
}

pub fn build_palette_widgets(cx: &mut Context) {
    let pair = AnotherLens::new(AppState::current_toolspec, AppState::current_layer);
    PaletteWidget::new(
        cx,
        current_palette_lens().then(ModuleAggregate::fg_tiles_palette),
        AppState::current_fg_tile,
        |cx, tile| {
            cx.emit(AppEvent::SelectPaletteTile { fg: true, tile });
        },
        AppState::current_fg_tile_other,
        |cx, other| cx.emit(AppEvent::SelectPaletteTileOther { fg: true, other }),
    )
    .bind(pair, |handle, pair| {
        let (toolspec, layer) = pair.get(&handle);
        handle.display(
            layer == Layer::FgTiles
                && (toolspec == ToolSpec::Pencil || toolspec == ToolSpec::Bucket),
        );
    });

    PaletteWidget::new(
        cx,
        current_palette_lens().then(ModuleAggregate::bg_tiles_palette),
        AppState::current_bg_tile,
        |cx, tile| cx.emit(AppEvent::SelectPaletteTile { fg: false, tile }),
        AppState::current_bg_tile_other,
        |cx, other| cx.emit(AppEvent::SelectPaletteTileOther { fg: false, other }),
    )
    .bind(pair, |handle, pair| {
        let (toolspec, layer) = pair.get(&handle);
        handle.display(
            layer == Layer::BgTiles
                && (toolspec == ToolSpec::Pencil || toolspec == ToolSpec::Bucket),
        );
    });

    PaletteWidget::new(
        cx,
        current_palette_lens().then(ModuleAggregate::entities_palette),
        AppState::current_entity,
        |cx, entity| cx.emit(AppEvent::SelectPaletteEntity { entity }),
        AppState::current_entity_other,
        |cx, other| cx.emit(AppEvent::SelectPaletteEntityOther { other }),
    )
    .bind(pair, |handle, pair| {
        let (toolspec, layer) = pair.get(&handle);
        handle.display(layer == Layer::Entities && toolspec == ToolSpec::Pencil);
    });

    PaletteWidget::new(
        cx,
        current_palette_lens().then(ModuleAggregate::triggers_palette),
        AppState::current_trigger,
        |cx, trigger| cx.emit(AppEvent::SelectPaletteTrigger { trigger }),
        AppState::current_trigger_other,
        |cx, other| cx.emit(AppEvent::SelectPaletteTriggerOther { other }),
    )
    .bind(pair, |handle, pair| {
        let (toolspec, layer) = pair.get(&handle);
        handle.display(layer == Layer::Triggers && toolspec == ToolSpec::Pencil);
    });

    PaletteWidget::new(
        cx,
        current_palette_lens().then(ModuleAggregate::decals_palette),
        AppState::current_decal,
        |cx, decal| cx.emit(AppEvent::SelectPaletteDecal { decal }),
        AppState::current_decal_other,
        |cx, other| cx.emit(AppEvent::SelectPaletteDecalOther { other }),
    )
    .bind(pair, |handle, pair| {
        let (toolspec, layer) = pair.get(&handle);
        handle.display(
            (layer == Layer::FgDecals || layer == Layer::BgDecals) && toolspec == ToolSpec::Pencil,
        );
    });

    Binding::new(cx, AppState::current_objtile, move |cx, objtile| {
        TilePaletteWidget::new(cx, objtile.get(cx), |cx, tile| {
            cx.emit(AppEvent::SelectPaletteObjectTile { tile })
        })
        .bind(pair, |handle, pair| {
            let (toolspec, layer) = pair.get(&handle);
            handle.display(layer == Layer::ObjectTiles && toolspec == ToolSpec::Pencil);
        })
        .min_height(Pixels(100.0))
        .min_width(Pixels(100.0))
        .height(Stretch(1.0))
        .width(Stretch(1.0));
    });
}

pub fn build_tweaker_widgets(cx: &mut Context) {
    Binding::new(cx, AppState::current_toolspec, |cx, tool_idx| {
        let tool_idx = tool_idx.get(cx);
        EntityTweakerWidget::new(cx).display(tool_idx == ToolSpec::Selection);
        RoomTweakerWidget::new(cx).display(tool_idx == ToolSpec::Room);
        StyleListWidget::new(cx).display(tool_idx == ToolSpec::Style);
        StyleTweakerWidget::new(cx).display(tool_idx == ToolSpec::Style);
    });
}
