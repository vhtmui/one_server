use chrono::{DateTime, Local, TimeZone};
use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, poll, read},
    prelude::*,
    widgets::{Block, Borders, Widget, WidgetRef},
};

pub struct FileMonitor {
    lunch_data: DateTime<Local>,
    files_got: usize,
    files_recorded: usize,
}

impl FileMonitor {
    pub fn new() -> Self {
        FileMonitor {
            lunch_data: Local::now(),
            files_got: 0,
            files_recorded: 0,
        }
    }
}

impl WidgetRef for FileMonitor {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::new().borders(Borders::ALL).title("FileMonitor");
        block.render(area, buf);
    }
}

pub struct Menu {
    
}