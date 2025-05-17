pub mod dir_scanner;
pub mod log_observer;
pub mod menujson;
pub mod registry;

pub use dir_scanner::*;
pub use log_observer::*;
pub use menujson::MENU_JSON;

use ratatui::style::Stylize;
use ratatui::symbols;

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;
use std::vec;

use chrono::Utc;
use ratatui::layout::Alignment;
use ratatui::text::{Line, Text};
use ratatui::widgets::{ListState, Paragraph, StatefulWidget, Tabs, Widget};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Direction, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, StatefulWidgetRef, WidgetRef},
};

use crate::my_widgets::{LogKind, render_input_popup};
use crate::{DirScannerEventKind, OneEvent};
use crate::{
    EventKind, TIME_ZONE,
    apps::AppAction::{self, *},
    my_widgets::{
        MyWidgets, dichotomize_area_with_midlines,
        menu::{MenuItem, MenuState, SerializableMenuItem},
    },
};

const TITLE_STYLE: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);

#[derive(Debug, PartialEq, Eq)]
enum CurrentArea {
    LogArea,
    ControlPanelArea,
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

pub struct SyncEngine {
    title: String,
    menu_struct: SerializableMenuItem,
    menu_state: RefCell<MenuState>,
    menu_selected_string: String,
    pub observer: LogObserver,
    pub scanner: DirScanner,
    log_list_state: RefCell<ListState>,
    log_tabs: usize,
    input_content: String,
    input_title: String,
    current_area: CurrentArea,
}

impl SyncEngine {
    pub fn new(title: String, path: PathBuf, log_size: usize) -> Self {
        let menu_struct = serde_json::from_str(MENU_JSON).unwrap();
        SyncEngine {
            title,
            menu_struct,
            menu_state: RefCell::new(MenuState::default()),
            menu_selected_string: String::new(),
            observer: LogObserver::new(path, log_size),
            scanner: DirScanner::new(log_size),
            log_list_state: RefCell::new(ListState::default()),
            log_tabs: 0,
            input_content: String::new(),
            input_title: String::new(),
            current_area: CurrentArea::ControlPanelArea,
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

    fn toggle_tabs(&mut self) {
        self.log_tabs = (self.log_tabs + 1) % 2;
    }

    fn clear_input(&mut self) {
        self.input_content.clear();
        self.input_title.clear();
        self.menu_selected_string.clear();
    }

    fn set_current_area(&mut self, area: CurrentArea) {
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

        let status = Line::from(format!("Status: {:?}", self.observer.get_status()));

        let lunch_time = Line::from(format!("Lunch time: {}", self.observer.get_lunch_time()));

        let elapsed_time = Line::from(format!(
            "Elapsed time: {}",
            self.observer.get_elapsed_time()
        ));

        let files_got = Line::from(format!("Files got: {}", self.observer.files_got()));

        let file_reading = Line::from(format!(
            "File reading: {}",
            self.observer.file_reading().display()
        ));

        let scanner_status = Line::from(format!("Scanner status: {:?}", self.scanner.get_status()));

        let files_recorded = Line::from(format!(
            "Files recorded: {:?}",
            self.observer.files_recorded()
        ));

        let text = Text::from(vec![
            status,
            lunch_time,
            elapsed_time,
            files_got,
            files_recorded,
            file_reading,
            scanner_status,
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

        let tabs_area = Rect {
            x: area.x + 1,
            y: area.y,
            width: area.width - 2,
            height: 1,
        };

        Tabs::new(vec!["observer", "scanner"])
            .style(Style::default().white())
            .highlight_style(Style::default().green().bg(Color::Yellow))
            .select(self.log_tabs)
            .divider(symbols::DOT)
            .render(tabs_area, buf);

        let log_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width - 1,
            height: area.height - 2,
        };

        self.render_logs(log_area, buf);
    }

    pub fn render_logs(&self, area: Rect, buf: &mut Buffer) {
        // 不应clone，会导致wrap_len状态无法保存到实例
        let list = if self.log_tabs == 0 {
            &mut self.observer.shared_state.lock().unwrap().logs
        } else {
            &mut self.scanner.shared_state.lock().unwrap().logs
        };

        StatefulWidget::render(list, area, buf, &mut *self.log_list_state.borrow_mut());
    }
}

impl WidgetRef for SyncEngine {
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
            render_input_popup(&self.input_content, area, buf, &self.input_title);
        }
    }
}

impl MyWidgets for SyncEngine {
    fn handle_event(&mut self, event: Event) -> Result<AppAction, std::io::Error> {
        // if in menu area
        match self.current_area {
            CurrentArea::ControlPanelArea => match event {
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if !self.menu_state.borrow().selected_indices.is_empty() {
                        match self.get_menu_result().as_str() {
                            "monitor-start" => {
                                self.observer.start_observer().unwrap();
                            }
                            "monitor-stop" => {
                                self.observer.stop_observer();
                            }
                            "scanner-start" => {
                                self.input_title = "Input path".to_string();
                                self.menu_selected_string = "scanner-start".to_string();
                                self.set_current_area(CurrentArea::InputArea);
                            }
                            "scanner-start-periodic" => {
                                self.input_title = "Input path and interval".to_string();
                                self.menu_selected_string = "scanner-start-periodic".to_string();
                                self.set_current_area(CurrentArea::InputArea);
                            }
                            _ => {}
                        };
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.menu_state.borrow_mut().select_up();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.menu_state.borrow_mut().select_down();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.menu_state.borrow_mut().select_left();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.menu_state.borrow_mut().select_right();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    return Ok(ToggleMenu);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Tab,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.toggle_area();
                }
                _ => {}
            },
            CurrentArea::LogArea => {
                if let Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    ..
                }) = event
                {
                    match code {
                        KeyCode::Left | KeyCode::Right => {
                            self.toggle_tabs();
                        }
                        KeyCode::Up => {
                            self.log_list_state.borrow_mut().scroll_up_by(1);
                        }
                        KeyCode::Down => {
                            self.log_list_state.borrow_mut().scroll_down_by(1);
                        }
                        KeyCode::Esc => {
                            return Ok(ToggleMenu);
                        }
                        KeyCode::Tab => {
                            self.toggle_area();
                        }
                        _ => {}
                    }
                }
            }
            CurrentArea::InputArea => match event {
                Event::Paste(s) => {
                    self.input_content.push_str(&s);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.input_content.push(c);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.input_content.pop();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => match self.menu_selected_string.as_str() {
                    "scanner-start" => {
                        self.scanner
                            .set_path(PathBuf::from(self.input_content.clone()));
                        self.scanner.start_scanner()?;

                        self.clear_input();
                        self.set_current_area(CurrentArea::ControlPanelArea);
                    }
                    "scanner-start-periodic" => {
                        self.scanner
                            .set_path(PathBuf::from(self.input_content.clone()));

                        self.clear_input();
                        self.input_title = "Input period (min)".to_string();
                        self.menu_selected_string = "scanner-start-periodic-with-delay".to_string();
                        self.set_current_area(CurrentArea::InputArea);
                    }
                    "scanner-start-periodic-with-delay" => {
                        match self.input_content.trim().parse::<u64>() {
                            Ok(val) => {
                                self.scanner
                                    .start_periodic_scan(Duration::from_secs(val * 60));
                            }
                            Err(_) => {
                                self.scanner.add_logs(OneEvent {
                                    time: Some(Utc::now().with_timezone(TIME_ZONE)),
                                    kind: EventKind::DirScannerEvent(DirScannerEventKind::Error),
                                    content: "Failed to parse input content".to_string(),
                                });
                            }
                        };
                        self.clear_input();
                        self.set_current_area(CurrentArea::ControlPanelArea);
                    }
                    "scanner-stop" => {
                        self.scanner.stop_periodic_scan();
                        self.set_current_area(CurrentArea::ControlPanelArea);
                    }
                    _ => {}
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    self.set_current_area(CurrentArea::ControlPanelArea);
                }
                _ => {}
            },
            _ => {}
        }

        Ok(Default)
    }

    fn get_logs_str(&self, kind: LogKind) -> Vec<String> {
        match kind {
            LogKind::All => {
                let mut logs = self.observer.get_logs_str();
                logs.extend(self.scanner.get_logs_str());
                logs
            }
            LogKind::Observer => self.observer.get_logs_str(),
            LogKind::Scanner => self.scanner.get_logs_str(),
        }
    }
}
