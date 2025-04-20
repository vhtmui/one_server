mod monitor;

pub use monitor::*;

use std::cell::RefCell;
use std::thread::sleep;

use ratatui::layout::Alignment;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, read},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, StatefulWidgetRef, Widget, WidgetRef},
};

use crate::{
    apps::AppAction::{self, *},
    my_widgets::{
        MyWidgets, dichotomize_area_with_midlines,
        menu::{MenuItem, MenuState, SerializableMenuItem},
    },
};

const TITLE_STYLE: Style = Style::new()
    .add_modifier(Modifier::REVERSED)
    .add_modifier(Modifier::BOLD);
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
        let mut result = vec![current.name.clone()];

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
        self.render_block("Control Panel".to_string(), area, buf);
        self.render_menu(area, buf);
    }

    pub fn render_status_area(&self, area: Rect, buf: &mut Buffer) {
        self.render_block("Status Area".to_string(), area, buf);
    }

    pub fn render_log_area(&self, area: Rect, buf: &mut Buffer) {
        self.render_block("Log Area".to_string(), area, buf);
    }

    pub fn start_monitor(&mut self) {
        self.monitor.start_monitor().unwrap();
    }

    pub fn render_block(&self, title: String, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .title(title)
            .title_style(TITLE_STYLE)
            .title_alignment(Alignment::Center);
        block.render(area, buf);
    }

    pub fn render_menu(&self, area: Rect, buf: &mut Buffer) {
        let json_data = MENU_JSON;
        let menu_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)].as_ref())
            .split(area)[1];

        let mut state = self.menu_state.borrow_mut();

        if let Ok(menu_item) = MenuItem::from_json(json_data) {
            StatefulWidgetRef::render_ref(&*menu_item.borrow(), menu_area, buf, &mut *state);
        }
    }
}

impl WidgetRef for FileMonitor {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let (left_area, midline, right_area) = dichotomize_area_with_midlines(
            area,
            Direction::Horizontal,
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        );

        let (left_up_area, left_midline, left_down_area) = dichotomize_area_with_midlines(
            left_area,
            Direction::Vertical,
            Constraint::Percentage(50),
            Constraint::Percentage(50),
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
                                self.start_monitor();
                            }
                            _ => {
                            }
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
