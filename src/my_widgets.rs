use std::thread::sleep;

use chrono::{DateTime, Local, TimeZone};
use crossterm::event;
use ratatui::{
    Frame,
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, poll, read},
    layout::Rect,
    widgets::{Block, Borders, List, Widget, WidgetRef, block::title},
};

use crate::app::{DEFAULT, EXIT_PROGRESS, TOGGLE_MENU};

pub trait MyWidgets: WidgetRef {
    fn handle_event(&self, event: Event) -> Result<char, std::io::Error>;
}

pub struct FileMonitor {
    title: String,
    lunch_datatime: DateTime<Local>,
    monitor_status: bool,
    files_got: usize,
    files_recorded: usize,
}

impl FileMonitor {
    pub fn new(title: String) -> Self {
        FileMonitor {
            title: title,
            lunch_datatime: Local::now(),
            monitor_status: false,
            files_got: 0,
            files_recorded: 0,
        }
    }
    pub fn start_monitor(&mut self) {
        
    }
}

impl WidgetRef for FileMonitor {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::new().borders(Borders::ALL).title(&*self.title);
        block.render(area, buf);
    }
}

impl MyWidgets for FileMonitor {
    fn handle_event(&self, event: Event) -> Result<char, std::io::Error> {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Release,
            ..
        }) = event
        {
            match code {
                KeyCode::Esc => {
                    return Ok(TOGGLE_MENU);
                }
                _ => {}
            }
        }

        Ok(DEFAULT)
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
