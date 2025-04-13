use std::collections::HashMap;
use std::io::Stdout;
use std::{ops::Deref, time::Duration};

use crate::my_widgets::FileMonitor;
use chrono::Local;
use ratatui::layout::Rect;
use ratatui::prelude::CrosstermBackend;
use ratatui::widgets::{self, HighlightSpacing, List, ListState, StatefulWidget};
use ratatui::{
    Frame, Terminal,
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, poll, read},
    style::{Modifier, Style, palette::tailwind::SLATE},
    widgets::{Block, Borders, Widget, WidgetRef},
};
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
pub struct Menu {
    show: bool,
    state: ListState,
}

pub struct Table {
    apps: HashMap<String, Box<dyn WidgetRef>>,
    current_app: String,
    menu: Menu,
}

impl Table {
    pub fn new() -> Self {
        Table {
            apps: HashMap::new(),
            current_app: String::new(),
            menu: Menu {
                show: false,
                state: ListState::default(),
            },
        }
    }

    pub fn add_widgets(mut self, name: String, widgets: Box<dyn WidgetRef>) -> Self {
        self.apps.insert(name, widgets);

        self
    }

    pub fn set_current_page(mut self, app: String) -> Self {
        self.current_app = app;
        self
    }

    pub fn toggle_menu(&mut self) {
        self.menu.show = !self.menu.show;
    }

    pub fn get_current_app(&self) -> &String {
        &self.current_app
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let data_time_now = Local::now();
        loop {
            terminal.draw(|frame| self.draw(frame)).unwrap();

            if poll(Duration::from_millis(0))? {
                let event = read()?;
                if !self.handle_event(event)? {
                    break;
                }
            } else {
                smol::future::yield_now().await;
            }
        }

        Ok(false)
    }
    pub fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    pub fn render_menu(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new().borders(Borders::ALL).title("Menu");

        let menu_list = List::new(self.apps.keys().map(AsRef::as_ref).collect::<Vec<&str>>())
            .block(block)
            .highlight_spacing(HighlightSpacing::WhenSelected)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">");

        StatefulWidget::render(menu_list, area, buf, &mut self.menu.state);
    }

    pub fn handle_event(&mut self, event: Event) -> Result<bool, Box<dyn std::error::Error>> {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Release,
            ..
        }) = event
        {
            match code {
                KeyCode::Esc => {
                    self.menu.show = !self.menu.show;
                }
                KeyCode::Enter => {}
                KeyCode::Char('q') => {
                    if self.menu.show {
                        return Ok(false);
                    }
                }
                _ => {}
            }
        }
        Ok(true)
    }
}

impl Widget for &mut Table {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        if let Some(widgets) = self.apps.get(&self.current_app) {
            widgets.render_ref(area, buf);
        }
        if self.menu.show {
            self.render_menu(get_center_rect(area, 0.5, 0.5), buf);
        }
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
