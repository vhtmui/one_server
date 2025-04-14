use std::thread::sleep;

use chrono::{DateTime, Local, TimeZone};
use crossterm::event;
use ratatui::{
    buffer::Buffer, crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyEventKind}, layout::Rect, widgets::{block::title, Block, Borders, List, Widget, WidgetRef}, Frame
};

pub trait HandleEvent {
    fn handle_event(&self, event: Event) -> Result<bool, Box<dyn std::error::Error>>;
}

pub struct FileMonitor {
    title: String,
    lunch_datatime: DateTime<Local>,
    files_got: usize,
    files_recorded: usize,
}

impl FileMonitor {
    pub fn new(title: String) -> Self {
        FileMonitor {
            title: title,
            lunch_datatime: Local::now(),
            files_got: 0,
            files_recorded: 0,
        }
    }
}

impl WidgetRef for FileMonitor {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::new().borders(Borders::ALL).title(&*self.title);
        block.render(area, buf);
    }
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