mod my_widgets;
mod app;

use macro_rules_attribute::apply;
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
};
use smol::{self, lock::futures::BarrierWait};
use smol_macros::main;

#[apply(main!)]
async fn main() {
    let mut terminal = ratatui::init();

    let app = Table::new();

    run_app(&mut terminal, &app).await.unwrap();

    ratatui::restore();
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &Table,
) -> Result<bool, Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| app.draw(f)).unwrap();

        if poll(Duration::from_millis(0))? {
            if !app::handle_event().unwrap() {
                break;
            };
        } else {
            smol::future::yield_now().await;
        }
    }

    Ok(false)
}
