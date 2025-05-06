use macro_rules_attribute::apply;
use ratatui::{
    Terminal,
    crossterm::{execute, terminal::{EnterAlternateScreen,enable_raw_mode} },
    prelude::CrosstermBackend,
    restore
    
};
use serde::Deserialize;
use serde_json;
use smol_macros::main;

use std::{fs, io::stdout};

fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        hook(info);
    }));
}

use one_server::{
    Config, add_widgets,
    apps::{Apps, file_monitor::FileMonitor},
};

#[apply(main!)]
async fn main() {
    set_panic_hook();
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen);
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    let app = Apps::new();

    let path: Config =
        serde_json::from_str(&fs::read_to_string("asset\\cfg.json").unwrap()).unwrap();

    let file_monitor = (
        String::from("file_monitor"),
        Box::new(FileMonitor::new(
            "file_monitor".to_string(),
            path.file_monitor.monitor_path,
            50,
        )),
    );

    add_widgets!(app, file_monitor)
        .set_current_app(0)
        .run(&mut terminal)
        .unwrap();
}

#[derive(Deserialize, Debug)]
struct Cfg {
    file_monitor: String,
}
