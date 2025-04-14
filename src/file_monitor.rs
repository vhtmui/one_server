use chrono::{DateTime, Local, TimeZone};
use notify::{Event, RecursiveMode, Result, Watcher};
use std::fs;
use std::path::Path;
use std::sync::mpsc::{self, channel};
use std::thread;
use std::time::{Duration, Instant};

pub struct Monitor {
    path: String,
    lunch_datetime: DateTime<Local>,
    life_time: Duration,
    service_status: bool,
    files_got: usize,
    files_recorded: usize,
}

impl Monitor {
    pub fn new(path: String) -> Self {
        Monitor {
            path,
            lunch_datetime: Local::now(),
            life_time: Duration::from_secs(0),
            service_status: false,
            files_got: 0,
            files_recorded: 0,
        }
    }

    pub fn start_monitor(&mut self) -> Result<()> {
        let path = self.path.clone();

        let handle = thread::spawn(move || {
            // 修改闭包返回类型为 Result<()>
            if let Err(e) = Self::inner_monitor(&path) {
                eprintln!("Error in file monitoring thread: {:?}", e);
                return Err(e); // 返回错误给主线程
            }
            Ok(())
        });

        // 等待子线程完成，并捕获可能的错误
        match handle.join() {
            Ok(result) => result,
            Err(_) => Err(notify::Error::generic("Thread panicked".into())),
        }
    }

    // 将核心逻辑提取到单独函数中
    fn inner_monitor(path: &str) -> Result<()> {
        let (tx, rx) = mpsc::channel::<Result<Event>>();

        // 使用 ? 操作符处理错误
        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(Path::new(path), RecursiveMode::Recursive)?;

        loop {
            match rx.recv() {
                Ok(event) => {
                    println!("File changed: {:?}", event);
                }
                Err(e) => {
                    eprintln!("Watch error: {:?}", e);
                    break; // 遇到接收错误时退出循环
                }
            }
        }

        Ok(())
    }

    // 示例分析函数，可以根据实际需求进行修改
    fn analyze_content(content: &str) -> String {
        // 这里可以添加具体的分析逻辑
        content.to_string()
    }
}
