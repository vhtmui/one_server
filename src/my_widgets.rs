use ratatui::{
    crossterm::event::KeyCode,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::WidgetRef,
};

use crate::apps::AppAction;

pub mod menu;
pub mod wrap_list;

pub trait MyWidgets: WidgetRef {
    fn handle_event(&mut self, code: KeyCode) -> Result<AppAction, std::io::Error>;
}

pub fn get_center_rect(area: Rect, width_percentage: f32, height_percentage: f32) -> Rect {
    if width_percentage > 0.0
        && width_percentage < 1.0
        && height_percentage > 0.0
        && height_percentage < 1.0
    {
        Rect {
            x: (area.width as f32 * (1.0 - width_percentage) * 0.5) as u16,
            y: (area.height as f32 * (1.0 - height_percentage) * 0.5) as u16,
            width: (area.width as f32 * width_percentage) as u16,
            height: (area.height as f32 * height_percentage) as u16,
        }
    } else {
        area
    }
}

pub fn dichotomize_area_with_midlines(
    area: Rect,
    direction: Direction,
    left_constraint: Constraint,
    right_constraint: Constraint,
    midline_width: u16,
) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(direction)
        .constraints(
            [
                left_constraint,
                Constraint::Length(midline_width),
                right_constraint,
            ]
            .as_ref(),
        )
        .split(area);

    (chunks[0], chunks[1], chunks[2])
}
