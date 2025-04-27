use crate::log;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self};
use std::time::Duration;

use chrono::format::OffsetFormat;
use chrono::{DateTime, FixedOffset, TimeDelta, TimeZone, Utc};
use notify::{Error, Event as NotifyEvent, EventKind, RecursiveMode, Result, Watcher};
use smol::fs;

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
    pub lunch_time: DateTime<FixedOffset>,
    pub elapsed_time: TimeDelta,
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
    files_watched: HashMap<PathBuf, FileWatchInfo>,
    files_got: usize,
    files_recorded: usize,
}

impl FileStatistics {
    pub fn new() -> Self {
        FileStatistics {
            files_watched: HashMap::new(),
            files_got: 0,
            files_recorded: 0,
        }
    }
}

pub struct FileWatchInfo {
    lastime_size_record: usize,
    lastime_byte_read_to: usize,
}

impl FileWatchInfo {
    pub fn new(lastime_size_record: usize, lastime_byte_read_to: usize) -> Self {
        FileWatchInfo {
            lastime_size_record,
            lastime_byte_read_to,
        }
    }
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
            lunch_time: DateTime::from_timestamp(0, 0)
                .unwrap()
                .with_timezone(TIME_ZONE), // Updated to DateTime<FixedOffset>
            elapsed_time: TimeDelta::zero(),
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
            locked_state.lunch_time = Utc::now().with_timezone(TIME_ZONE);
            locked_state.set_status(Running);
        }

        let time = Utc::now().with_timezone(TIME_ZONE);
        self.shared_state.lock().unwrap().lunch_time = time;

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
                    self.shared_state.lock().unwrap().reset_time();
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

            ss.elapsed_time = Utc::now().with_timezone(TIME_ZONE) - ss.lunch_time;

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
                            let path = event.paths[0].clone();
                        }
                        EventKind::Create(_) => {
                            let path = event.paths[0].clone();
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

    pub fn get_lunch_time(&self) -> String {
        self.shared_state
            .lock()
            .unwrap()
            .lunch_time
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    pub fn get_elapsed_time(&self) -> String {
        let ss = self.shared_state.lock().unwrap();
        format!(
            "{}h {}m {}s",
            ss.elapsed_time.num_seconds() / 3600,
            (ss.elapsed_time.num_seconds() % 3600) / 60,
            ss.elapsed_time.num_seconds() % 60
        )
    }

    pub fn get_status(&self) -> MonitorStatus {
        self.shared_state.lock().unwrap().status.clone()
    }
    pub fn files_got(&self) -> usize {
        self.shared_state.lock().unwrap().file_statistic.files_got
    }

    pub fn files_recorded(&self) -> usize {
        self.shared_state.lock().unwrap().file_statistic.files_recorded
    }
}

impl SharedState {
    fn add_event(&mut self, event: MonitorEvent) {
        self.logs.add_raw_item(event);
    }

    pub fn reset_time(&mut self) {
        self.lunch_time = DateTime::from_timestamp(0, 0)
            .unwrap()
            .with_timezone(TIME_ZONE);
        self.elapsed_time = TimeDelta::zero();
    }

    fn set_status(&mut self, status: MonitorStatus) {
        self.status = status;
    }

    async fn add_file_watchinfo(&mut self, path: PathBuf) {
        let file_size = fs::metadata(&path).await.unwrap().len();

        let file_watch_info = FileWatchInfo::new(file_size as usize, 0);

        self.file_statistic
            .files_watched
            .insert(path, file_watch_info);
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
