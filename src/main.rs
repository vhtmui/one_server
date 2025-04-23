use macro_rules_attribute::apply;
use smol_macros::main;

use one_server::{
    add_widgets,
    apps::{Apps, file_monitor::FileMonitor},
};

#[apply(main!)]
async fn main() {
    let mut terminal = ratatui::init();

    let app = Apps::new();

    let path = r#"asset\f_monitor_test"#.to_string();
    let file_monitor = (
        String::from("file_monitor"),
        Box::new(FileMonitor::new("file_monitor".to_string(), path, 50)),
    );

    add_widgets!(app, file_monitor)
        .set_current_app(0)
        .run(&mut terminal)
        .await
        .unwrap();

    ratatui::restore();
}
