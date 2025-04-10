use std::time::Duration;

use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, poll, read},
    widgets::{Block, Borders, Widget},
};

enum Pages {
    Server,
}

pub struct App {
    current_page: Pages,
    menu_show: bool,
}

impl App {
    pub fn new() -> Self {
        App {
            current_page: Pages::Server,
            menu_show: false,
        }
    }

    pub fn toggle_menu(&mut self) {
        self.menu_show = !self.menu_show;
    }

    pub fn current_page(&self) -> &Pages {
        &self.current_page
    }

    pub fn set_current_page(&mut self, page: Pages) {
        self.current_page = page;
    }

    pub fn draw(&self, f: &mut Frame) {
        match self.current_page {
            Pages::Server => {
                f.render_widget(server_page(), f.area());
            }
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
