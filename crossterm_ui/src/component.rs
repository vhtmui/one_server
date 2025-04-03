use crate::tools::{self, clear_area};
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, WindowSize, size,
    },
};
use smol;
use std::{default, io::stdout};

/// A unit of position or size.
pub struct Dimension {
    pub x: u16,
    pub y: u16,
}

pub struct Selection {
    items: Vec<String>,
    position: Dimension,
    default_selected: usize,
}

impl Selection {
    pub fn new(items: Vec<String>, position: Dimension, default_selected: usize) -> Self {
        Self {
            items,
            position,
            default_selected,
        }
    }
    pub fn new_with_default(items: Vec<String>) -> Self {
        Self {
            items,
            position: Dimension { x: 1, y: 1 },
            default_selected: 1,
        }
    }
    fn get_size(&self) -> Dimension {
        let (x, _) = size().unwrap();
        Dimension {
            x,
            y: self.items.len() as u16,
        }
    }
    fn clear_self(&self) {
        let mut stdout = stdout();

        let rows = self.get_size().y;

        let clear_area();
    }
    pub async fn run() {
        let mut stdout = stdout();
    }
}
