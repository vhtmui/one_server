use smol;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{Write, stdout};

/// 多选菜单结构体
pub struct MultiSelectMenu {
    options: Vec<String>,
    selected: Vec<bool>,
}

impl MultiSelectMenu {
    /// 创建一个新的多选菜单
    pub fn new(options: Vec<String>, default_selected: Vec<usize>) -> Self {
        let mut selected = vec![false; options.len()];
        for index in default_selected {
            if index < options.len() {
                selected[index] = true;
            }
        }
        Self { options, selected }
    }

    /// 异步运行菜单并返回用户选择的结果
    pub async fn run(&mut self) -> Vec<usize> {
        terminal::enable_raw_mode().unwrap();
        let mut stdout = stdout();
        let mut current_index = 0;

        loop {
            // 非阻塞读取事件
            if event::poll(std::time::Duration::from_millis(100)).unwrap() {
                if let Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    ..
                }) = event::read().unwrap()
                {
                    match code {
                        KeyCode::Up => {
                            if current_index > 0 {
                                current_index -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if current_index < self.options.len() - 1 {
                                current_index += 1;
                            }
                        }
                        KeyCode::Char(' ') => {
                            self.selected[current_index] = !self.selected[current_index];
                        }
                        KeyCode::Enter => {
                            break;
                        }
                        KeyCode::Char('q') | KeyCode::Esc => {
                            terminal::disable_raw_mode().unwrap();
                            execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0)).unwrap();
                            return vec![];
                        }
                        _ => {}
                    }

                    // 清除屏幕并重绘菜单
                    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0)).unwrap();
                    for (i, option) in self.options.iter().enumerate() {
                        if i == current_index {
                            // 高亮当前光标所在行
                            execute!(
                                stdout,
                                SetForegroundColor(Color::Green),
                                Print(format!(
                                    "> [{}] {}\n",
                                    if self.selected[i] { "X" } else { " " },
                                    option
                                )),
                                ResetColor
                            )
                            .unwrap();
                        } else {
                            // 普通显示其他行
                            execute!(
                                stdout,
                                Print(format!(
                                    "  [{}] {}\n",
                                    if self.selected[i] { "X" } else { " " },
                                    option
                                ))
                            )
                            .unwrap();
                        }
                    }
                }
            } else {
                // 如果没有事件发生，继续等待
                smol::future::yield_now().await;
            }
        }

        // 恢复终端状态
        terminal::disable_raw_mode().unwrap();
        execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0)).unwrap();

        // 返回用户选择的索引
        self.selected
            .iter()
            .enumerate()
            .filter_map(|(i, &selected)| if selected { Some(i) } else { None })
            .collect()
    }
}