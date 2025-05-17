use macro_rules_attribute::apply;

use ratatui::crossterm::execute;
use smol_macros::main;

use one_server::*;

#[apply(main!)]
async fn main() {
    execute!(
        std::io::stdout(),
        ratatui::crossterm::terminal::SetTitle("One Server 文件同步")
    )
    .unwrap();

    param::handle_params();
}
