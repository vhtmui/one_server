use macro_rules_attribute::apply;
use smol_macros::{main, test};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, ScrollUp, SetSize, size},
};
use smol::{self, fs::read};
use std::io::{self, stdout};
use crossterm_ui;

#[apply(main)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = stdout();

    Screan::run("Alter");

    let page1 = Page::new();

    page1.add_element(Element::new());

    page1.draw();



    Ok(())
}
