// use chrono::{DateTime, FixedOffset, Local, TimeZone, Utc};
// use notify::{Event, RecursiveMode, Result, Watcher};
// use std::collections::VecDeque;
// use std::fs;
// use std::path::{Path, PathBuf};
// use std::sync::mpsc::{self, channel};
// use std::thread;
// use std::time::{Duration, Instant};

// pub struct Monitor {
//     path: String,
//     lunch_time: Option<DateTime<FixedOffset>>,
//     elapsed_time: Duration,
//     status: MonitorStatus,
//     file_analyzer: FileAnalyzer,
//     events: VecDeque<MonitorEvent>,
//     handle: Option<thread::JoinHandle<Result<()>>>,
// }

// impl Monitor {
//     pub fn new(path: String) -> Self {
//         Monitor {
//             path,
//             lunch_time: None,
//             elapsed_time: Duration::from_secs(0),
//             status: MonitorStatus::Stopped,
//             file_analyzer: FileAnalyzer::default(),
//             events: VecDeque::with_capacity(10),
//             handle: None,
//         }
//     }

//     pub fn start_monitor(&'static mut self) -> Result<()> {
//         self.lunch_time = Some(Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap()));

//         let path = self.path.clone();

//         let handle = thread::spawn(move || {
//             if let Err(e) = self.inner_monitor(&path) {
//                 eprintln!("Error in file monitoring thread: {:?}", e);
//                 return Err(e);
//             }
//             Ok(())
//         });

//         self.handle = Some(handle);
//         self.status = MonitorStatus::Running;

//         Ok(())
//     }

//     fn inner_monitor(&mut self, path: &str) -> Result<()> {
//         let (tx, rx) = mpsc::channel::<Result<Event>>();

//         let mut watcher = notify::recommended_watcher(tx)?;

//         watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;

//         loop {
//             match rx.recv() {
//                 Ok(event) => {

//                 }
//                 Err(e) => {
//                     eprintln!("Watch error: {:?}", e);
//                     break;
//                 }
//             }
//         }
//         Ok(())
//     }

//     pub fn add_event(&mut self, event: MonitorEvent) {
//         if self.events.len() == 10 {
//             self.events.pop_front();
//         }
//         self.events.push_back(event);
//     }

//     fn analyze_content(content: &str) -> String {
//         content.to_string()
//     }
// }

// pub enum MonitorStatus {
//     Running,
//     Stopped,
//     Paused,
//     Error,
// }

// pub struct MonitorEvent {
//     time: Option<DateTime<FixedOffset>>,
//     event_type: MonitorEventType,
//     message: String,
// }

// #[derive(Default)]
// pub struct FileAnalyzer {
//     files_watched: Vec<WatchFileInfo>,
//     files_got: usize,
//     files_recorded: usize,
// }

// pub struct WatchFileInfo{
//     path: PathBuf,
//     last_size: usize,
//     last_byte_read_to: usize,
// }

// pub enum MonitorEventType {
//     CreatedFile,
//     ModifiedFile,
//     DeletedFile,
// }

use chrono::{DateTime, FixedOffset, Local, TimeZone, Utc};
use notify::{Event, RecursiveMode, Result, Watcher};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

pub struct Monitor {
    path: String,
    lunch_time: Option<DateTime<FixedOffset>>,
    elapsed_time: Duration,
    status: MonitorStatus,
    file_analyzer: FileAnalyzer,
    events: VecDeque<MonitorEvent>,
    handle: Option<thread::JoinHandle<()>>, // 修改为无返回值
}

impl Monitor {
    pub fn new(path: String) -> Self {
        Monitor {
            path,
            lunch_time: None,
            elapsed_time: Duration::from_secs(0),
            status: MonitorStatus::Stopped,
            file_analyzer: FileAnalyzer::default(),
            events: VecDeque::with_capacity(10),
            handle: None,
        }
    }

    pub fn start_monitor(&'static mut self) -> Result<()> {
        let monitor = Arc::new(Mutex::new(&mut *self)); // 使用 Arc 和 Mutex 包装 self
        let cloned_monitor = Arc::clone(&monitor);

        monitor.lock().unwrap().lunch_time =
            Some(Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap()));

        let path = monitor.lock().unwrap().path.clone();

        let handle = thread::spawn(move || {
            if let Err(e) = Monitor::inner_monitor(cloned_monitor, &path) {
                eprintln!("Error in file monitoring thread: {:?}", e);
            }
        });

        monitor.lock().unwrap().handle = Some(handle);
        monitor.lock().unwrap().status = MonitorStatus::Running;

        Ok(())
    }

    fn inner_monitor(monitor: Arc<Mutex<&mut Monitor>>, path: &str) -> Result<()> {
        let (tx, rx) = mpsc::channel::<Result<Event>>();

        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;

        loop {
            match rx.recv() {
                Ok(event) => {
                    let mut locked_monitor = monitor.lock().unwrap();
                    locked_monitor.handle_event(event?); // 调用事件处理方法
                }
                Err(e) => {
                    eprintln!("Watch error: {:?}", e);
                    break;
                }
            }
        }
        Ok(())
    }

    fn handle_event(&mut self, event: Event) {
        match event.kind {
            notify::event::EventKind::Create(_) => {
                self.add_event(MonitorEvent {
                    time: Some(
                        Local::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap()),
                    ),
                    event_type: MonitorEventType::CreatedFile,
                    message: format!("File created: {:?}", event.paths),
                });
            }
            notify::event::EventKind::Modify(_) => {
                self.add_event(MonitorEvent {
                    time: Some(
                        Local::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap()),
                    ),
                    event_type: MonitorEventType::ModifiedFile,
                    message: format!("File modified: {:?}", event.paths),
                });
            }
            notify::event::EventKind::Remove(_) => {
                self.add_event(MonitorEvent {
                    time: Some(
                        Local::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap()),
                    ),
                    event_type: MonitorEventType::DeletedFile,
                    message: format!("File deleted: {:?}", event.paths),
                });
            }
            _ => {}
        }
    }

    pub fn add_event(&mut self, event: MonitorEvent) {
        if self.events.len() == 10 {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    fn analyze_content(content: &str) -> String {
        content.to_string()
    }
}

pub enum MonitorStatus {
    Running,
    Stopped,
    Paused,
    Error,
}

pub struct MonitorEvent {
    time: Option<DateTime<FixedOffset>>,
    event_type: MonitorEventType,
    message: String,
}

#[derive(Default)]
pub struct FileAnalyzer {
    files_watched: Vec<WatchFileInfo>,
    files_got: usize,
    files_recorded: usize,
}

pub struct WatchFileInfo {
    path: PathBuf,
    last_size: usize,
    last_byte_read_to: usize,
}

pub enum MonitorEventType {
    CreatedFile,
    ModifiedFile,
    DeletedFile,
}
