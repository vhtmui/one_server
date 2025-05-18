use ratatui::crossterm::execute;

use one_server::*;

#[tokio::main]
async fn main() {
    execute!(
        std::io::stdout(),
        ratatui::crossterm::terminal::SetTitle("One Server 文件同步")
    )
    .unwrap();

    param::handle_params();
}
