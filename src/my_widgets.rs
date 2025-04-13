use chrono::{DateTime, Local, TimeZone};
use crossterm::event;
use ratatui::{
    Frame,
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, poll, read},
    layout::Rect,
    widgets::{Block, Borders, List, Widget, WidgetRef},
};

pub struct FileMonitor {
    lunch_datatime: DateTime<Local>,
    files_got: usize,
    files_recorded: usize,
}

impl FileMonitor {
    pub fn new() -> Self {
        FileMonitor {
            lunch_datatime: Local::now(),
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

