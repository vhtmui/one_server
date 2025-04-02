use macro_rules_attribute::apply;
use smol_macros::{main, test};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, ScrollUp, SetSize, size},
};
use crossterm_ui;
use smol::{self, fs::read};
use std::io::{self, Write, stdout};

#[apply(main)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = stdout();

    execute!(stdout, EnterAlternateScreen, Print("asdfasdfasf\nsdfasdfasfsd\nasfasfdsfas"))?;
    terminal::enable_raw_mode()?;

    loop {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event::read().unwrap()
        {
            match code {
                KeyCode::Esc => {
                    // terminal::disable_raw_mode()?;
                }
                KeyCode::Enter => {
                    stdout.flush()?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
