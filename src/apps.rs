use std::io::Stdout;
use std::time::Instant;
use std::{ops::Deref, time::Duration};

use ratatui::layout::Rect;
use ratatui::prelude::CrosstermBackend;
use ratatui::style::Styled;
use ratatui::widgets::{self, HighlightSpacing, List, ListState, StatefulWidget};
use ratatui::{
    Frame, Terminal,
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, poll, read},
    style::{Modifier, Style, palette::tailwind::SLATE},
    widgets::{Block, Borders, Widget, WidgetRef},
};

use crate::{
    apps::AppAction::*,
    my_widgets::{MyWidgets, get_center_rect},
};

pub mod file_monitor;

pub const MENU_SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
pub const MENU_HIGHLIGHT_STYLE: Style = Style::new()
    .bg(SLATE.c800)
    .fg(ratatui::style::Color::Green)
    .add_modifier(Modifier::BOLD);
pub const MENU_STYLE: Style = Style::new().bg(SLATE.c600).add_modifier(Modifier::BOLD);
const THROTTLE_DURATION: Duration = Duration::from_millis(100);

#[derive(PartialEq, Eq)]
pub enum AppAction {
    Default,
    ToggleMenu,
    ExitProgress,
}

pub struct AppsMenu {
    show: bool,
    state: ListState,
}

pub struct Apps {
    apps: Vec<(String, Box<dyn MyWidgets>)>,
    current_app: usize,
    menu: AppsMenu,
    last_event_time: Instant,
}

impl Apps {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Apps {
            apps: Vec::new(),
            current_app: 0,
            menu: AppsMenu { show: false, state },
            last_event_time: Instant::now(),
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<bool, std::io::Error> {
        // let data_time_now = Local::now();
        'app: loop {
            terminal
                .draw(|frame| frame.render_widget(&mut *self, frame.area()))
                .unwrap();

            if poll(Duration::from_millis(0))? {
                // 渲染计算量过大时限制操作频率。实际应优先优化计算缓存
                // let mut events = Vec::new();

                // while poll(Duration::ZERO)? {
                //     events.push(read()?);
                // }

                // let mut events_iter = events.iter();

                // for _ in 1..=2 {
                //     if let Some(event) = events_iter.next() {
                //         if let Ok(ExitProgress) = self.handle_event(event.clone()) {
                //             break 'app;
                //         }
                //     }
                // }
                let event = read()?;

                if let Ok(ExitProgress) = self.handle_event(event.clone()) {
                    break 'app;
                }
            }
        }

        Ok(true)
    }

    pub fn render_menu(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .borders(Borders::ALL)
            .title("Menu")
            .set_style(MENU_STYLE);

        let apps = self.get_apps();

        let menu_list = List::new(apps.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
            .block(block)
            .highlight_spacing(HighlightSpacing::WhenSelected)
            .highlight_style(MENU_SELECTED_STYLE)
            .highlight_symbol(">");

        StatefulWidget::render(menu_list, area, buf, &mut self.menu.state);
    }

    pub fn handle_event(&mut self, event: Event) -> Result<AppAction, std::io::Error> {
        // if self.last_event_time.elapsed() < THROTTLE_DURATION {
        //     return Ok(Default);
        // }
        // self.last_event_time = Instant::now();

        let result = if self.menu.show {
            self.handle_menu_event(event)
        } else {
            self.get_current_app().handle_event(event)
        };

        match result {
            Ok(ExitProgress) => Ok(ExitProgress),
            Ok(ToggleMenu) => {
                self.toggle_menu();
                Ok(Default)
            }
            Ok(Default) => Ok(Default),
            Err(e) => Err(e),
        }
    }

    fn handle_menu_event(&mut self, event: Event) -> Result<AppAction, std::io::Error> {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
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
                        return Ok(ExitProgress);
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

        Ok(Default)
    }

    pub fn add_widgets(mut self, name: String, widgets: Box<dyn MyWidgets>) -> Self {
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

    pub fn get_current_app(&mut self) -> &mut Box<dyn MyWidgets> {
        &mut self.apps[self.current_app].1
    }

    pub fn get_apps(&self) -> Vec<String> {
        self.apps.iter().map(|x| x.0.clone()).collect()
    }

    pub fn clear_area(area: Rect, buf: &mut Buffer) {
        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                buf[(x, y)].reset();
            }
        }
    }
}

impl Widget for &mut Apps {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        // Render the current app
        let current_app = &*self.apps[self.current_app].1;
        current_app.render_ref(area, buf);

        // Render the menu if show
        if self.menu.show {
            let area = get_center_rect(area, 0.5, 0.5);

            Apps::clear_area(area, buf);
            self.render_menu(area, buf);
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
