mod app;
mod file_monitor;
mod my_widgets;

use macro_rules_attribute::apply;
use smol_macros::main;

use crate::{app::Table, file_monitor::FileMonitor};

#[apply(main!)]
async fn main() {
    let mut terminal = ratatui::init();

    let app = Table::new();

    let path = "C:\\Users\\Administrator\\Desktop\\session.log".to_string();
    let file_monitor = (
        String::from("file_monitor"),
        Box::new(FileMonitor::new("file_monitor".to_string(), path)),
    );

    add_widgets!(app, file_monitor)
        .set_current_app(0)
        .run(&mut terminal)
        .await
        .unwrap();

    ratatui::restore();
}
