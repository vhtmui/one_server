use crate::tools::clear_area;
use crossterm::{
    cursor::{self},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::size,
};
use smol;
use std::{io::Write, io::stdout};

/// A unit of position or size.
pub struct XY(u16, u16);

pub struct Selection {
    items: Vec<String>,
    position: XY,
    default_selected: usize,
}

impl Selection {
    pub fn new(items: Vec<String>, position: XY, default_selected: usize) -> Self {
        Self {
            items,
            position,
            default_selected,
        }
    }

    pub fn new_with_default(items: Vec<String>) -> Self {
        Self {
            items,
            position: XY(1, 1),
            default_selected: 0,
        }
    }

    pub fn set_position(&mut self, position: XY) {
        self.position = position;
    }

    fn get_size(&self) -> XY {
        let (x, _) = size().unwrap();
        XY(x, self.items.len() as u16)
    }

    fn clear_self(&self) {
        let start = &self.position;
        let size = &self.get_size();

        clear_area(start, size);
    }

    fn print_item(&self, index: usize, selected: bool) {
        let mut stdout = stdout();
        let color;
        let item;

        if selected {
            color = Color::Green;
            item = format!("> {}", self.items[index]);
        } else {
            color = Color::Reset;
            item = format!("  {}", self.items[index]);
        }

        queue!(
            stdout,
            cursor::MoveTo(self.position.0, self.position.1 + index as u16),
            SetForegroundColor(color),
            Print(item),
            ResetColor,
        )
        .unwrap();

        stdout.flush().unwrap();
    }

    pub async fn run(&mut self) {
        self.clear_self(); 

        for index in 0..=self.items.len() - 1 {
            if index == self.default_selected {
                self.print_item(index, true);
            } else {
                self.print_item(index, false);
            }
        }

        event::read().unwrap();

        loop {
            if event::poll(std::time::Duration::from_millis(0)).unwrap() {
                if let Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    ..
                }) = event::read().unwrap()
                {
                    match code {
                        KeyCode::Up => {
                            if self.default_selected > 0 {
                                self.default_selected -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if self.default_selected < self.items.len() - 1 {
                                self.default_selected += 1;
                            }
                        }
                        KeyCode::Enter => {
                            break;
                        }
                        _ => {}
                    }

                    self.clear_self(); 

                    for index in 0..=self.items.len() - 1 {
                        if index == self.default_selected {
                            self.print_item(index, true);
                        } else {
                            self.print_item(index, false);
                        }
                    }
                }
            } else {
                smol::future::yield_now().await;
            }
        }
    }
}
