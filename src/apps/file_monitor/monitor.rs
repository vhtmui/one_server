use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self};
use std::time::Duration;

use chrono::{DateTime, FixedOffset, Utc};
use notify::{Event as NotifyEvent, RecursiveMode, Result as NotifyResult, Watcher};

use crate::apps::file_monitor::MonitorStatus::*;

pub struct Monitor {
    path: String,
    shared_state: Arc<Mutex<SharedState>>,
    handle: Option<thread::JoinHandle<()>>,
}

struct SharedState {
    lunch_time: Option<DateTime<FixedOffset>>,
    elapsed_time: Duration,
    status: MonitorStatus,
    file_analyzer: FileAnalyzer,
    events: VecDeque<MonitorEvent>,
}

#[derive(Clone, PartialEq, Eq)]
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
    time: Option<DateTime<FixedOffset>>,
    event_type: MonitorEventType,
    message: String,
}

pub enum MonitorEventType {
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

    pub fn start_monitor(&mut self) -> NotifyResult<()> {
        let mut locked_state = self.shared_state.lock().unwrap();
        locked_state.lunch_time =
            Some(Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap()));
        locked_state.status = Running;

        let path = self.path.clone();
        let cloned_shared_state = Arc::clone(&self.shared_state);
        let handle = thread::spawn(move || {
            if let Err(e) = Monitor::inner_monitor(cloned_shared_state, &path) {
                eprintln!("Error in file monitoring thread: {:?}", e);
            }
        });

        self.handle = Some(handle);

        Ok(())
    }

    pub fn stop_monitor(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().expect("Failed to join file monitoring thread");
        }
    }

    fn inner_monitor(shared_state: Arc<Mutex<SharedState>>, path: &str) -> NotifyResult<()> {
        let (tx, rx) = mpsc::channel::<NotifyResult<NotifyEvent>>();

        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;

        loop {
            match rx.recv() {
                Ok(event) => {
                    print!("Event: {:?}\n", event);
                }
                Err(e) => {
                    eprintln!("Watch error: {:?}", e);
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

    pub fn get_status_text(&self) -> MonitorStatus {
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
