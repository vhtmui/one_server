use std::collections::HashMap;
use std::io::Stdout;
use std::{ops::Deref, time::Duration};

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

use crate::my_widgets::get_center_rect;

const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

pub struct Menu {
    show: bool,
    state: ListState,
}

pub struct Table {
    apps: Vec<(String, Box<dyn WidgetRef>)>,
    current_app: usize,
    menu: Menu,
}

impl Table {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Table {
            apps: Vec::new(),
            current_app: 0,
            menu: Menu { show: false, state },
        }
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let data_time_now = Local::now();
        loop {
            terminal
                .draw(|frame| frame.render_widget(&mut *self, frame.area()))
                .unwrap();

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

    pub fn render_menu(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new().borders(Borders::ALL).title("Menu");

        let apps = self.get_apps();

        let menu_list = List::new(apps.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
            .block(block)
            .highlight_spacing(HighlightSpacing::WhenSelected)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">");

        StatefulWidget::render(menu_list, area, buf, &mut self.menu.state);
    }

    pub fn handle_event(&mut self, event: Event) -> Result<bool, Box<dyn std::error::Error>> {
        if self.menu.show {
            if let Ok(result) = self.handle_menu_event(event) {
                return Ok(result);
            }
        } else {
            if let Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Release,
                ..
            }) = event
            {
                match code {
                    KeyCode::Esc => self.toggle_menu(),
                    KeyCode::Enter => {}
                    _ => {}
                }
            }
        }
        Ok(true)
    }
    fn handle_menu_event(&mut self, event: Event) -> Result<bool, Box<dyn std::error::Error>> {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Release,
            ..
        }) = event
        {
            match code {
                KeyCode::Esc => self.toggle_menu(),
                KeyCode::Enter => {
                    if let Some(index) = self.menu.state.selected() {
                        self.current_app = index;
                        self.toggle_menu();
                    }
                }
                KeyCode::Char('q') => {
                    if self.menu.show {
                        return Ok(false);
                    }
                }
                KeyCode::Up => {
                    if self.menu.show {
                        self.menu.state.select_previous();
                    }
                }
                KeyCode::Down => {
                    if self.menu.show {
                        self.menu.state.select_next();
                    }
                }
                _ => {}
            }
        }
        Ok(true)
    }

    pub fn add_widgets(mut self, name: String, widgets: Box<dyn WidgetRef>) -> Self {
        self.apps.push((name, widgets));
        self
    }

    pub fn set_current_app(mut self, index: usize) -> Self {
        self.current_app = index;
        self
    }

    pub fn toggle_menu(&mut self) {
        self.menu.show = !self.menu.show;
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn get_apps(&self) -> Vec<String> {
        self.apps.iter().map(|x| x.0.clone()).collect()
    }
}

impl Widget for &mut Table {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        // Render the current app
        let _ = &self.apps[self.current_app].1.render_ref(area, buf);

        // Render the menu if show
        if self.menu.show {
            self.render_menu(get_center_rect(area, 0.5, 0.5), buf);
        }
    }
}

#[macro_export]
macro_rules!  add_widgets {
    ($table:expr, $($widget:expr),*) => {
        $table$(
            .add_widgets($widget.0, $widget.1)
        )*
    };
}
