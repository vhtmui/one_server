use crate::log;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self};
use std::time::Duration;

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use notify::{Error, Event as NotifyEvent, EventKind, RecursiveMode, Result, Watcher};

use crate::{
    apps::file_monitor::{MonitorStatus::*, monitor},
    my_widgets::wrap_list::WrapList,
};

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
    pub file_statistic: FileStatistics,
    pub logs: WrapList,
    pub should_stop: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MonitorStatus {
    Running,
    Stopped,
    Paused,
    Error,
}

#[derive(Default)]
pub struct FileStatistics {
    files_watched: Vec<FileWhatchInfo>,
    files_got: usize,
    files_recorded: usize,
}

pub struct FileWhatchInfo {
    path: PathBuf,
    last_size_record: usize,
    last_byte_read_to: usize,
}

#[derive(Clone, Debug)]
pub struct MonitorEvent {
    pub time: Option<DateTime<FixedOffset>>,
    pub event_type: MonitorEventType,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum MonitorEventType {
    StopMonitor,
    Error,
    CreatedFile,
    ModifiedFile,
    DeletedFile,
    Info,
}

impl Monitor {
    pub fn new(path: String, log_size: usize) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            lunch_time: None,
            elapsed_time: Duration::from_secs(0),
            status: Stopped,
            file_statistic: FileStatistics::default(),
            logs: WrapList::new(log_size),
            should_stop: false,
        }));

        Monitor {
            path,
            shared_state,
            handle: None,
        }
    }

    pub fn start_monitor(&mut self) -> Result<()> {
        if self.shared_state.lock().unwrap().status == Running {
            log!(
                self.shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                MonitorEventType::Error,
                "Monitor is already running".to_string()
            );
            return Ok(());
        }

        let path = self.path.clone();
        if !Path::new(&path).exists() {
            let current_path = std::env::current_dir()?;

            log!(
                self.shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                MonitorEventType::Error,
                format!(
                    "Path does not exist, current path: {}",
                    current_path.display()
                )
            );
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

        log!(
            self.shared_state,
            Utc::now().with_timezone(TIME_ZONE),
            MonitorEventType::Info,
            "Monitor started".to_string()
        );
        Ok(())
    }

    pub fn stop_monitor(&mut self) {
        self.shared_state.lock().unwrap().should_stop = true;

        thread::sleep(Duration::from_millis(800));

        if let Some(handle) = self.handle.take() {
            match handle.is_finished() {
                true => {
                    log!(
                        self.shared_state,
                        Utc::now().with_timezone(TIME_ZONE),
                        MonitorEventType::StopMonitor,
                        "Monitor stopped.".to_string()
                    );
                }
                false => {
                    log!(
                        self.shared_state,
                        Utc::now().with_timezone(TIME_ZONE),
                        MonitorEventType::Error,
                        "Monitor is already stopped.".to_string()
                    );
                }
            }
        }
    }

    fn inner_monitor(shared_state: Arc<Mutex<SharedState>>, path: &str) -> Result<()> {
        let (tx, rx) = mpsc::channel::<Result<NotifyEvent>>();

        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;

        loop {
            let mut ss = shared_state.lock().unwrap();
            if ss.should_stop {
                ss.status = Stopped;
                ss.should_stop = false;
                break;
            }
            drop(ss);

            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(event) => {
                    let event = event.unwrap();
                    log!(
                        shared_state,
                        Utc::now().with_timezone(TIME_ZONE),
                        MonitorEventType::ModifiedFile,
                        format!("Notify event: {:?}, {:?}", event.kind, event.paths)
                    );

                    match event.kind {
                        EventKind::Modify(_) => {
                            Self::do_something();
                        }
                        _ => {}
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(e) => {
                    log!(
                        shared_state,
                        Utc::now().with_timezone(TIME_ZONE),
                        MonitorEventType::Error,
                        format!("Error: {:?}", e)
                    );
                    break;
                }
            }
        }
        drop(watcher);
        Ok(())
    }

    pub fn get_status(&self) -> MonitorStatus {
        self.shared_state.lock().unwrap().status.clone()
    }

    pub fn do_something() {}
}

impl SharedState {
    fn add_event(&mut self, event: MonitorEvent) {
        self.logs.add_raw_item(event);
    }
}

#[macro_export]
macro_rules! log {
    ($shared_state:expr, $time:expr, $event_type:expr, $message:expr $(,)* ) => {
        $shared_state.lock().unwrap().add_event(MonitorEvent {
            time: Some($time),
            event_type: $event_type,
            message: $message,
        })
    };
}
