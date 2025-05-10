use macro_rules_attribute::apply;
use ratatui::{
    Terminal,
    crossterm::{
        execute,
        terminal::{EnterAlternateScreen, enable_raw_mode},
    },
    prelude::CrosstermBackend,
    restore,
};
use smol_macros::main;

use std::io::stdout;

fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        hook(info);
    }));
}

use one_server::{
    apps::{Apps, file_sync_manager::SyncEngine},
    *,
};

#[apply(main!)]
async fn main() {
    param::handle_params();

    set_panic_hook();
    enable_raw_mode().unwrap();
    execute!(stdout(), EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    let app = Apps::new();

    let path = load_config().file_monitor.monitor_path;

    let file_monitor = (
        String::from("file_monitor"),
        Box::new(SyncEngine::new("file_monitor".to_string(), path, 50)),
    );

    add_widgets!(app, file_monitor)
        .set_current_app(0)
        .run(&mut terminal)
        .unwrap();
}
