mod monitor;

pub use monitor::*;

use hyphenation::{Language, Load, Standard};
use std::cell::RefCell;
use textwrap::{Options, WordSplitter, fill};

use ratatui::layout::Alignment;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, read},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, StatefulWidgetRef, Widget, WidgetRef},
};
use textwrap;

use crate::{
    apps::{
        AppAction::{self, *},
        file_monitor::monitor::MonitorStatus::*,
    },
    my_widgets::{
        MyWidgets, dichotomize_area_with_midlines,
        menu::{MenuItem, MenuState, SerializableMenuItem},
    },
};

const TITLE_STYLE: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
const MENU_JSON: &str = r#"
{
    "name": "Monitor Menu",
    "content": "This is a menu of file monitor.",
    "children": [
        {
            "name": "monitor",
            "content": "This is a description.",
            "children": [
                {
                    "name": "start",
                    "content": "This is a description of Skyrim.",
                    "children": []
                },
                {
                    "name": "stop",
                    "content": "This is a description of Skyrim.",
                    "children": []
                }
            ]
        }
    ]
}
"#;

pub struct FileMonitor {
    title: String,
    menu_state: RefCell<MenuState>,
    monitor: Monitor,
    menu_struct: SerializableMenuItem,
}

impl FileMonitor {
    pub fn new(title: String, path: String) -> Self {
        let menu_struct = serde_json::from_str(MENU_JSON).unwrap();
        FileMonitor {
            menu_state: RefCell::new(MenuState::default()),
            title: title,
            monitor: Monitor::new(path),
            menu_struct,
        }
    }

    pub fn get_menu_result(&self) -> String {
        let indices = self.menu_state.borrow().selected_indices.clone();
        let mut current = &self.menu_struct;
        let mut result = Vec::new();

        for index in indices {
            if index >= current.children.len() {
                panic!(
                    "Index {} out of bounds while get menu item string vector with {} children",
                    index,
                    current.children.len()
                );
            }
            current = &current.children[index];
            result.push(current.name.clone());
        }

        result.join("-")
    }

    pub fn render_control_panel(&self, area: Rect, buf: &mut Buffer) {
        let menu_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)].as_ref())
            .split(area)[1];

        self.render_block("Control Panel".to_string(), area, buf);
        self.render_menu(menu_area, buf);
    }

    pub fn render_status_area(&self, area: Rect, buf: &mut Buffer) {
        self.render_block("Status Area".to_string(), area, buf);
    }

    pub fn render_log_area(&self, area: Rect, buf: &mut Buffer) {
        let log_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)].as_ref())
            .split(area)[1];

        self.render_block("Log Area".to_string(), area, buf);
        self.render_logs(log_area, buf);
    }

    pub fn render_block(&self, title: String, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .title(title)
            .title_style(TITLE_STYLE)
            .title_alignment(Alignment::Center);
        block.render(area, buf);
    }

    pub fn render_menu(&self, area: Rect, buf: &mut Buffer) {
        let mut state = self.menu_state.borrow_mut();

        if let Ok(menu_item) = MenuItem::from_json(MENU_JSON) {
            StatefulWidgetRef::render_ref(&*menu_item.borrow(), area, buf, &mut *state);
        }
    }

    pub fn render_logs(&self, area: Rect, buf: &mut Buffer) {
        let events = &self.monitor.shared_state.lock().unwrap().events;

        let items: Vec<ListItem> = events
            .iter()
            .rev()
            .map(|e| {
                let (prefix, color) = match e.event_type {
                    MonitorEventType::Error => ("[ERR]  ", Color::Red),
                    MonitorEventType::CreatedFile => ("[CREATE]", Color::Green),
                    MonitorEventType::ModifiedFile => ("[MODIFY]", Color::Blue),
                    MonitorEventType::DeletedFile => ("[DELETE]", Color::Magenta),
                    MonitorEventType::StopMonitor => ("[STOP]", Color::Yellow),
                    MonitorEventType::Info => ("[INFO]  ", Color::White),
                };

                let time_str = e
                    .time
                    .map(|t| t.format("%H:%M:%S").to_string())
                    .unwrap_or_else(|| "--:--:--".into());

                let text = vec![prefix, " ", time_str.as_str(), " ", &e.message].join("");

                let dictionary = Standard::from_embedded(Language::EnglishUS).unwrap();
                let options = textwrap::Options::new(area.width as usize)
                    .word_splitter(WordSplitter::Hyphenation(dictionary));

                let lines: Vec<Line> = textwrap::wrap(text.as_str(), &options)
                    .iter()
                    .map(|line| Line::from(line.to_string()))
                    .collect();

                let item = ListItem::new(Text::from(lines));
                item
            })
            .collect();

        List::new(items)
            .block(Block::default().borders(Borders::NONE))
            .render(area, buf);
    }
}

impl WidgetRef for FileMonitor {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let (left_area, midline, right_area) = dichotomize_area_with_midlines(
            area,
            Direction::Horizontal,
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        );

        let (left_up_area, left_midline, left_down_area) = dichotomize_area_with_midlines(
            left_area,
            Direction::Vertical,
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        );

        Block::default().borders(Borders::LEFT).render(midline, buf);
        Block::default()
            .borders(Borders::TOP)
            .render(left_midline, buf);

        self.render_control_panel(left_up_area, buf);
        self.render_status_area(left_down_area, buf);
        self.render_log_area(right_area, buf);
    }
}

impl MyWidgets for FileMonitor {
    fn handle_event(&mut self, event: Event) -> Result<AppAction, std::io::Error> {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Release,
            ..
        }) = event
        {
            match code {
                KeyCode::Esc => {
                    return Ok(ToggleMenu);
                }
                KeyCode::Enter => {
                    if !self.menu_state.borrow().selected_indices.is_empty() {
                        match self.get_menu_result().as_str() {
                            "monitor-start" => {
                                if self.monitor.get_status() != Running {
                                    self.monitor.start_monitor();
                                }
                            }
                            "monitor-stop" => {
                                if self.monitor.get_status() != Stopped {
                                    self.monitor.stop_monitor();
                                }
                            }
                            _ => {}
                        };
                    }
                }
                KeyCode::Up => {
                    self.menu_state.borrow_mut().select_up();
                }
                KeyCode::Down => {
                    self.menu_state.borrow_mut().select_down();
                }
                KeyCode::Left => {
                    self.menu_state.borrow_mut().select_left();
                }
                KeyCode::Right => {
                    self.menu_state.borrow_mut().select_right();
                }
                _ => {}
            }
        }

        Ok(Default)
    }
}
