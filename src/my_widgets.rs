use ratatui::{
    buffer::Buffer,
    crossterm::event::Event,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    widgets::{Block, Clear, Paragraph, Widget, WidgetRef},
};

use crate::apps::AppAction;

pub mod menu;
pub mod wrap_list;

pub enum LogKind {
    All,
    Observer,
    Scanner,
}

pub trait MyWidgets: WidgetRef {
    fn handle_event(&mut self, event: Event) -> Result<AppAction, std::io::Error>;
    fn get_logs_str(&self, kind: LogKind) -> Vec<String>;
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

pub fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

pub fn render_input_popup<'a>(content: &'a str, area: Rect, buf: &mut Buffer, title: &str) {
    let area = center(area, Constraint::Percentage(50), Constraint::Length(3));
    let popup = Paragraph::new(content).block(Block::bordered().title(title));
    Clear.render(area, buf);
    popup.render(area, buf);
}
