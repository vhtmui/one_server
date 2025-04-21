use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self};
use std::time::Duration;

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use notify::{Error, Event as NotifyEvent, RecursiveMode, Result, Watcher};

use crate::apps::file_monitor::MonitorStatus::*;

const TIME_ZONE: &FixedOffset = &FixedOffset::east_opt(8 * 3600).unwrap();

pub struct Monitor {
    pub path: String,
    pub shared_state: Arc<Mutex<SharedState>>,
    pub handle: Option<thread::JoinHandle<Result<()>>>,
}

pub struct SharedState {
    pub lunch_time: Option<DateTime<FixedOffset>>,
    pub elapsed_time: Duration,
    pub status: MonitorStatus,
    pub file_analyzer: FileAnalyzer,
    pub events: VecDeque<MonitorEvent>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MonitorStatus {
    Running,
    Stopped,
    Paused,
    Error,
}

#[derive(Default)]
pub struct FileAnalyzer {
    files_watched: Vec<FileWhatchInfo>,
    files_got: usize,
    files_recorded: usize,
}

pub struct FileWhatchInfo {
    path: PathBuf,
    last_size: usize,
    last_byte_read_to: usize,
}

pub struct MonitorEvent {
    pub time: Option<DateTime<FixedOffset>>,
    pub event_type: MonitorEventType,
    pub message: String,
}

pub enum MonitorEventType {
    Error,
    CreatedFile,
    ModifiedFile,
    DeletedFile,
}

impl Monitor {
    pub fn new(path: String) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            lunch_time: None,
            elapsed_time: Duration::from_secs(0),
            status: Stopped,
            file_analyzer: FileAnalyzer::default(),
            events: VecDeque::with_capacity(10),
        }));

        Monitor {
            path,
            shared_state,
            handle: None,
        }
    }

    pub fn start_monitor(&mut self) -> Result<()> {
        if self.shared_state.lock().unwrap().status == Running {
            self.add_event(MonitorEvent {
                time: Some(Utc::now().with_timezone(TIME_ZONE)),
                event_type: MonitorEventType::Error,
                message: "Monitor is already running".to_string(),
            });
            return Ok(());
        }

        let path = self.path.clone();
        if !Path::new(&path).exists() {
            self.shared_state.lock().unwrap().add_event(MonitorEvent {
                time: Some(Utc::now().with_timezone(TIME_ZONE)),
                event_type: MonitorEventType::Error,
                message: "Path does not exist".to_string(),
            });
            return Ok(());
        }

        {
            let mut locked_state = self.shared_state.lock().unwrap();
            locked_state.lunch_time = Some(Utc::now().with_timezone(TIME_ZONE));
            locked_state.status = Running;
        }

        let cloned_shared_state = Arc::clone(&self.shared_state);
        let handle = thread::spawn(move || Monitor::inner_monitor(cloned_shared_state, &path));

        self.handle = Some(handle);

        self.add_event(MonitorEvent {
            time: Some(Utc::now().with_timezone(TIME_ZONE)),
            event_type: MonitorEventType::CreatedFile,
            message: "Monitor started".to_string(),
        });
        Ok(())
    }

    pub fn stop_monitor(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .expect("Failed to join file monitoring thread")
                .unwrap();
        }
    }

    fn inner_monitor(shared_state: Arc<Mutex<SharedState>>, path: &str) -> Result<()> {
        let (tx, rx) = mpsc::channel::<Result<NotifyEvent>>();

        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;

        loop {
            match rx.recv() {
                Ok(event) => {
                    shared_state.lock().unwrap().add_event(MonitorEvent {
                        time: Some(Utc::now().with_timezone(TIME_ZONE)),
                        event_type: MonitorEventType::ModifiedFile,
                        message: format!("File modified: {:?}", event),
                    });
                }
                Err(e) => {
                    shared_state.lock().unwrap().add_event(MonitorEvent {
                        time: Some(Utc::now().with_timezone(TIME_ZONE)),
                        event_type: MonitorEventType::Error,
                        message: format!("Error: {:?}", e),
                    });
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn add_event(&mut self, event: MonitorEvent) {
        let mut locked_state = self.shared_state.lock().unwrap();
        if locked_state.events.len() == 10 {
            locked_state.events.pop_front();
        }
        locked_state.events.push_back(event);
    }

    fn analyze_content(content: &str) -> String {
        content.to_string()
    }

    pub fn get_status(&self) -> MonitorStatus {
        self.shared_state.lock().unwrap().status.clone()
    }
}

impl SharedState {
    fn add_event(&mut self, event: MonitorEvent) {
        if self.events.len() == 10 {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
}
