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

    let app = Table::new();
    let file_monitor = (String::from("file_monitor"), Box::new(FileMonitor::new()));
    let file_monitor2 = (String::from("file_monitor2"), Box::new(FileMonitor::new()));
    let file_monitor3 = (String::from("file_monitor3"), Box::new(FileMonitor::new()));

    add_widgets!(app, file_monitor, file_monitor2, file_monitor3)
        .set_current_page(String::from("file_monitor"))
        .run(&mut terminal)
        .await
        .unwrap();

    ratatui::restore();
}
