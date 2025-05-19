use std::{fs::OpenOptions, io::Write};

use ratatui::{crossterm::execute, restore};

use one_server::*;

#[tokio::main]
async fn main() {
    set_panic_hook();

    execute!(
        std::io::stdout(),
        ratatui::crossterm::terminal::SetTitle("One Server 文件同步")
    )
    .unwrap();

    param::handle_params();
}

fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("panic.log")
        {
            let now = chrono::Local::now();
            let msg = format!("{}: {:?}", now.format("%Y-%m-%d %H:%M:%S"), info);
            let _ = file.write_all(msg.as_bytes());
        }

        restore();

        hook(info);
    }));
}
