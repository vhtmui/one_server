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
    handle: Option<thread::JoinHandle<Result<()>>>,
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
            handle: None,
        }
    }

    pub fn start_monitor(&mut self) -> Result<()> {
        let path = self.path.clone();

        let handle = thread::spawn(move || {
            if let Err(e) = Self::inner_monitor(&path) {
                eprintln!("Error in file monitoring thread: {:?}", e);
                return Err(e);
            }
            Ok(())
        });

        self.handle = Some(handle);

        Ok(())
    }

    fn inner_monitor(path: &str) -> Result<()> {
        let (tx, rx) = mpsc::channel::<Result<Event>>();

        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(Path::new(path), RecursiveMode::Recursive)?;

        loop {
            match rx.recv() {
                Ok(event) => {
                    println!("File changed: {:?}", event);
                }
                Err(e) => {
                    eprintln!("Watch error: {:?}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    fn analyze_content(content: &str) -> String {
        content.to_string()
    }
}
