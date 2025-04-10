use std::collections::HashMap;
use std::{ops::Deref, time::Duration};

use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, poll, read},
    widgets::{Block, Borders, WidgetRef Widget},
};

pub struct Table<T>
where
    T: WidgetRef,
{
    apps: HashMap<String, T>,
    current_app: String,
    menu_show: bool,
}

impl<T> Table<T>
where
    T: Widget,
{
    pub fn new() -> Self {
        Table {
            apps: HashMap::new(),
            current_app: String::new(),
            menu_show: false,
        }
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

    pub fn draw(&self, f: &mut Frame) {
        if let Some(&widget) = self.apps.get(&self.current_app) {
            f.render_widget(widget, f.area());
        }
    }
}

fn server_page() -> impl Widget {
    let block = Block::new().title("Server Page").borders(Borders::ALL);
    block
}

pub fn handle_event() -> Result<bool, Box<dyn std::error::Error>> {
    if let Event::Key(KeyEvent {
        code,
        kind: KeyEventKind::Release,
        ..
    }) = read()?
    {
        match code {
            KeyCode::Esc => {
                return Ok(false);
            }
            KeyCode::Enter => {}
            _ => {}
        }
    }
    Ok(true)
}
