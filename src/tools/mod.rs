pub mod hand;
pub mod pencil;

use crate::app_state::{AppState, AppEvent};
use crate::units::*;

use vizia::*;

pub trait Tool {
    fn name(&self) -> &'static str;

    fn event(&mut self, event: &WindowEvent, state: &AppState, cx: &Context) -> Vec<AppEvent>;

    fn switch_on(&mut self) { }
}

const SCROLL_SENSITIVITY: f32 = 35.0;

pub fn generic_scroll(event: &WindowEvent, state: &AppState, cx: &Context) -> Vec<AppEvent> {
    match event {
        WindowEvent::MouseScroll(x, y) if cx.modifiers.contains(Modifiers::CTRL) => {
            let screen_pt = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
            vec![AppEvent::Zoom { delta: y.exp(), focus: state.transform.inverse().unwrap().transform_point(screen_pt) }]
        }
        WindowEvent::MouseScroll(x, y) if !cx.modifiers.contains(Modifiers::CTRL) => {
            let (x, y) = if cx.modifiers.contains(Modifiers::SHIFT) {(y, x)} else {(x, y)};
            let screen_vec = ScreenVector::new(-*x, *y) * SCROLL_SENSITIVITY;
            let map_vec = state.transform.inverse().unwrap().transform_vector(screen_vec);
            vec![AppEvent::Pan { delta: map_vec }]
        }
        _ => vec![]
    }
}
