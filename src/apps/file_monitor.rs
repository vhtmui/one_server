pub mod maintainer;
pub mod monitor;

pub use monitor::*;

use std::cell::RefCell;
use std::path::PathBuf;
use std::vec;

use hyphenation::{Language, Load, Standard};
use ratatui::layout::Alignment;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, StatefulWidget};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, read},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, StatefulWidgetRef, Widget, WidgetRef},
};
use textwrap::{Options, WordSplitter, fill};

use crate::my_widgets::{self, menu, render_input_popup};
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

use super::MENU_HIGHLIGHT_STYLE;

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
        },
        {
            "name": "scanner",
            "content": "This is a description of scanner.",
            "children": [
                {
                    "name": "start",
                    "content": "This is a description of Skyrim.",
                    "children": []
                },
                {
                    "name": "stop(Developing)",
                    "content": "This is a description of Skyrim.",
                    "children": []

                }
            ]
        }
    ]
}
"#;

#[derive(Debug, PartialEq, Eq)]
enum CurrentArea {
    LogArea,
    ControlPanelArea,
    StatusArea,
    InputArea,
}

impl CurrentArea {
    fn toggle(&mut self) {
        match self {
            CurrentArea::LogArea => *self = CurrentArea::ControlPanelArea,
            CurrentArea::ControlPanelArea => *self = CurrentArea::LogArea,
            _ => {}
        }
    }

    fn set_current_area(&mut self, area: CurrentArea) {
        *self = area;
    }
}

pub struct FileMonitor {
    title: String,
    menu_struct: SerializableMenuItem,
    menu_state: RefCell<MenuState>,
    monitor: Monitor,
    log_list_state: RefCell<ListState>,
    input_content: String,
    current_area: CurrentArea,
}

impl FileMonitor {
    pub fn new(title: String, path: PathBuf, log_size: usize) -> Self {
        let menu_struct = serde_json::from_str(MENU_JSON).unwrap();
        FileMonitor {
            menu_state: RefCell::new(MenuState::default()),
            title,
            menu_struct,
            monitor: Monitor::new(path, log_size),
            log_list_state: RefCell::new(ListState::default()),
            current_area: CurrentArea::ControlPanelArea,
            input_content: String::new(),
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

    pub fn toggle_area(&mut self) {
        self.current_area.toggle();
    }

    pub fn set_current_area(&mut self, area: CurrentArea) {
        self.current_area.set_current_area(area);
    }

    pub fn render_control_panel(&self, area: Rect, buf: &mut Buffer, if_highlight: bool) {
        let mut state = self.menu_state.borrow_mut();

        if let Ok(menu_item) = MenuItem::from_json(MENU_JSON) {
            let block = Block::default()
                .borders(if if_highlight {
                    Borders::ALL
                } else {
                    Borders::NONE
                })
                .title("Control Panel")
                .title_style(TITLE_STYLE)
                .title_alignment(Alignment::Center);

            menu_item.borrow_mut().set_block(block);
            StatefulWidgetRef::render_ref(&*menu_item.borrow(), area, buf, &mut *state);
        }
    }

    pub fn render_status_area(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::NONE)
            .title("Status Area")
            .title_style(TITLE_STYLE)
            .title_alignment(Alignment::Center);

        let status = Line::from(format!("Status: {:?}", self.monitor.get_status()));

        let lunch_time = Line::from(format!("Lunch time: {}", self.monitor.get_lunch_time()));

        let elapsed_time = Line::from(format!("Elapsed time: {}", self.monitor.get_elapsed_time()));

        let files_got = Line::from(format!("Files got: {:?}", self.monitor.files_got()));

        let file_reading = Line::from(format!("File reading: {:?}", self.monitor.file_reading()));

        let files_recorded = Line::from(format!(
            "Files recorded: {:?}",
            self.monitor.files_recorded()
        ));

        let text = Text::from(vec![
            status,
            lunch_time,
            elapsed_time,
            files_got,
            files_recorded,
            file_reading,
        ]);

        Paragraph::new(text).block(block).render_ref(area, buf);
    }

    pub fn render_log_area(&self, area: Rect, buf: &mut Buffer, if_highlight: bool) {
        let block = Block::default()
            .borders(if if_highlight {
                Borders::ALL
            } else {
                Borders::NONE
            })
            .title("Log Area")
            .title_style(TITLE_STYLE)
            .title_alignment(Alignment::Center);
        block.render_ref(area, buf);

        let log_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width - 2,
            height: area.height - 2,
        };

        self.render_logs(log_area, buf);
    }

    pub fn render_logs(&self, area: Rect, buf: &mut Buffer) {
        let list = &mut self.monitor.shared_state.lock().unwrap().logs;

        StatefulWidget::render(list, area, buf, &mut *self.log_list_state.borrow_mut());
    }
}

impl WidgetRef for FileMonitor {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let (left_area, _midline, right_area) = dichotomize_area_with_midlines(
            area,
            Direction::Horizontal,
            Constraint::Percentage(30),
            Constraint::Percentage(70),
            0,
        );

        let (left_up_area, _left_midline, left_down_area) = dichotomize_area_with_midlines(
            left_area,
            Direction::Vertical,
            Constraint::Percentage(30),
            Constraint::Percentage(70),
            0,
        );

        self.render_control_panel(
            left_up_area,
            buf,
            self.current_area == CurrentArea::ControlPanelArea,
        );
        self.render_status_area(left_down_area, buf);
        self.render_log_area(right_area, buf, self.current_area == CurrentArea::LogArea);

        if self.current_area == CurrentArea::InputArea {
            render_input_popup(&self.input_content, area, buf);
        }
    }
}

impl MyWidgets for FileMonitor {
    fn handle_event(&mut self, code: KeyCode) -> Result<AppAction, std::io::Error> {
        match code {
            KeyCode::Esc => {
                return Ok(ToggleMenu);
            }
            KeyCode::Tab => {
                self.toggle_area();
            }
            code => {
                // if in menu area
                match self.current_area {
                    CurrentArea::ControlPanelArea => match code {
                        KeyCode::Enter => {
                            if !self.menu_state.borrow().selected_indices.is_empty() {
                                match self.get_menu_result().as_str() {
                                    "monitor-start" => {
                                        if self.monitor.get_status() != Running {
                                            self.monitor.start_monitor().unwrap();
                                        }
                                    }
                                    "monitor-stop" => {
                                        if self.monitor.get_status() != Stopped {
                                            self.monitor.stop_monitor();
                                        }
                                    }
                                    "scanner-start" => {
                                        if self.monitor.get_scanner_status() != "Running" {
                                            self.set_current_area(CurrentArea::InputArea);
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
                    },
                    CurrentArea::LogArea => match code {
                        KeyCode::Up => {
                            self.log_list_state.borrow_mut().scroll_up_by(1);
                        }
                        KeyCode::Down => {
                            self.log_list_state.borrow_mut().scroll_down_by(1);
                        }
                        _ => {}
                    },
                    CurrentArea::InputArea => {
                        match code {
                            KeyCode::Char(c) => {
                                self.input_content.push(c);
                            }
                            KeyCode::Backspace => {
                                self.input_content.pop();
                            }
                            KeyCode::Enter => {
                                self.monitor.start_scanner(PathBuf::from(self.input_content.clone()))?;
                                self.set_current_area(CurrentArea::LogArea);
                            }
                            KeyCode::Esc => {
                                self.set_current_area(CurrentArea::LogArea);
                            }
                            _ => {
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(Default)
    }
}
