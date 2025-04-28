use macro_rules_attribute::apply;
use serde::Deserialize;
use serde_json;
use smol_macros::main;
use std::fs;

use one_server::{
    add_widgets,
    apps::{Apps, file_monitor::FileMonitor},
};

#[apply(main!)]
async fn main() {
    let mut terminal = ratatui::init();

    let app = Apps::new();

    let path: Cfg =
        serde_json::from_str(&fs::read_to_string("asset\\cfg.json").unwrap()).unwrap();

    let file_monitor = (
        String::from("file_monitor"),
        Box::new(FileMonitor::new("file_monitor".to_string(), path.file_monitor_path, 50)),
    );

    add_widgets!(app, file_monitor)
        .set_current_app(0)
        .run(&mut terminal)
        .await
        .unwrap();

    ratatui::restore();
}

#[derive(Deserialize, Debug)]
struct Cfg {
    file_monitor_path: String,
}
