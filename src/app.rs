use std::collections::HashMap;
use std::io::Stdout;
use std::{ops::Deref, time::Duration};

use crate::my_widgets::FileMonitor;
use chrono::Local;
use ratatui::widgets;
use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, poll, read},
    prelude::*,
    widgets::{Block, Borders, Widget, WidgetRef},
};

pub struct Table {
    apps: HashMap<String, Box<dyn WidgetRef>>,
    current_app: String,
    menu_show: bool,
}

impl Table {
    pub fn new() -> Self {
        Table {
            apps: HashMap::new(),
            current_app: String::new(),
            menu_show: false,
        }
    }

    pub fn add_widgets(&mut self, name: String, widgets: Box<dyn WidgetRef>) {
        self.apps.insert(name, widgets);
    }

    pub fn toggle_menu(&mut self) {
        self.menu_show = !self.menu_show;
    }

    pub fn current_app(&self) -> &String {
        &self.current_app
    }

    pub fn set_current_page(&mut self, app: String) {
        self.current_app = app;
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
    pub fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
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
                    self.menu_show = !self.menu_show;
                }
                KeyCode::Enter => {}
                KeyCode::Char('q') => {
                    if self.menu_show {
                        return Ok(false);
                    }
                }
                _ => {}
            }
        }
        Ok(true)
    }
}

impl Widget for &Table {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        if let Some(widgets) = self.apps.get(&self.current_app) {
            widgets.render_ref(area, buf);
        }
        if self.menu_show {
            Block::default()
                .borders(Borders::ALL)
                .render(get_center_rect(area, 0.5, 0.5), buf);
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
