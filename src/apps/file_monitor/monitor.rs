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
    pub launch_time: DateTime<FixedOffset>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileWatchInfo {
    lastime_size: usize,
    current_size: usize,
}

impl FileWatchInfo {
    pub fn new(lastime_size: usize, current_size: usize) -> Self {
        FileWatchInfo {
            lastime_size,
            current_size,
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
            launch_time: DateTime::from_timestamp(0, 0)
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

        self.set_lunch_time();
        self.set_status(Running);

        let time = Utc::now().with_timezone(TIME_ZONE);
        self.shared_state.lock().unwrap().launch_time = time;

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
                    self.reset_time();
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

            ss.elapsed_time = Utc::now().with_timezone(TIME_ZONE) - ss.launch_time;

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

                            let update_result = smol::block_on(async {
                                shared_state.lock().unwrap().add_file_watchinfo(path).await
                            });

                            if let Some(info) = update_result {
                                log!(
                                    shared_state,
                                    Utc::now().with_timezone(TIME_ZONE),
                                    MonitorEventType::Info,
                                    format!(
                                        "File watched updated from {} bytes to {}",
                                        info.lastime_size, info.current_size
                                    )
                                );

                                if info.current_size > info.lastime_size {}
                            }
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

    pub fn set_lunch_time(&self) {
        self.shared_state.lock().unwrap().launch_time = Utc::now().with_timezone(TIME_ZONE);
    }

    pub fn get_lunch_time(&self) -> String {
        self.shared_state
            .lock()
            .unwrap()
            .launch_time
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

    pub fn reset_time(&self) {
        let mut ss = self.shared_state.lock().unwrap();
        ss.launch_time = DateTime::from_timestamp(0, 0)
            .unwrap()
            .with_timezone(TIME_ZONE);
        ss.elapsed_time = TimeDelta::zero();
    }

    pub fn get_status(&self) -> MonitorStatus {
        self.shared_state.lock().unwrap().status.clone()
    }

    pub fn files_got(&self) -> usize {
        self.shared_state.lock().unwrap().file_statistic.files_got
    }

    pub fn files_recorded(&self) -> usize {
        self.shared_state
            .lock()
            .unwrap()
            .file_statistic
            .files_recorded
    }

    pub fn set_status(&self, status: MonitorStatus) {
        self.shared_state.lock().unwrap().status = status;
    }
}

impl SharedState {
    fn add_event(&mut self, event: MonitorEvent) {
        self.logs.add_raw_item(event);
    }

    async fn add_file_watchinfo(&mut self, path: PathBuf) -> Option<FileWatchInfo> {
        let file_size = fs::metadata(&path).await.unwrap().len();

        let file_watch_info = if let Some(info) = self.file_statistic.files_watched.get(&path) {
            FileWatchInfo::new(info.current_size, file_size as usize)
        } else {
            FileWatchInfo::new(0, file_size as usize)
        };
        self.file_statistic
            .files_watched
            .insert(path, file_watch_info.clone());

        Some(file_watch_info)
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
