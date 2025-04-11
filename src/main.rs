mod app;
mod my_widgets;

use macro_rules_attribute::apply;
use my_widgets::FileMonitor;
use std::{
    io::{Stdout, stdout},
    time::Duration,
};

use ::crossterm::terminal::{self, enable_raw_mode};
use app::Table;
use ratatui::{
    Terminal,
    crossterm::{
        self,
        event::{EnableMouseCapture, poll},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode},
    },
    prelude::CrosstermBackend,
    widgets::{Widget, WidgetRef},
};
use smol::{self, lock::futures::BarrierWait};
use smol_macros::main;

#[apply(main!)]
async fn main() {
    let mut terminal = ratatui::init();

    let mut app = Table::new();
    let file_monitor = Box::new(FileMonitor::new());

    app.add_widgets(String::from("file_monitor"), file_monitor);
    app.set_current_page(String::from("file_monitor"));

    app.run(&mut terminal).await.unwrap();

    ratatui::restore();
}
