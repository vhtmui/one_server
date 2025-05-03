use crate::log;

use std::{
    collections::HashMap,
    panic,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use futures;
use notify::{Event as NotifyEvent, EventKind, RecursiveMode, Result, Watcher};
use smol::{
    fs,
    future::{self, FutureExt},
    io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom},
    pin,
    stream::{self, StreamExt},
};

use crate::{apps::file_monitor::MonitorStatus::*, my_widgets::wrap_list::WrapList};

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
    file_reading: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FileWatchInfo {
    last_read_pos: u64,
    file_size: u64,
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
                .with_timezone(TIME_ZONE),
            elapsed_time: TimeDelta::zero(),
            status: Stopped,
            file_statistic: FileStatistics::default(),
            logs: WrapList::new(log_size),
        }));

        Monitor {
            path,
            shared_state,
            handle: None,
        }
    }

    pub fn stop_monitor(&mut self) {
        self.shared_state
            .lock()
            .unwrap()
            .set_status(MonitorStatus::Stopped);
        thread::sleep(Duration::from_millis(800));

        if let Some(handle) = self.handle.take() {
            if handle.is_finished() {
                self.reset_time();
                log!(
                    self.shared_state,
                    Utc::now().with_timezone(TIME_ZONE),
                    MonitorEventType::StopMonitor,
                    "Monitor is stopping.".to_string()
                );
            } else {
                log!(
                    self.shared_state,
                    Utc::now().with_timezone(TIME_ZONE),
                    MonitorEventType::Error,
                    "Monitor doesn't stop.".to_string()
                );
            }
        }
    }

    pub fn start_monitor(&mut self) -> Result<()> {
        let ss = self.shared_state.lock().unwrap();
        if ss.status == Running {
            log!(
                self.shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                MonitorEventType::Error,
                "Monitor is already running".to_string()
            );
            return Ok(());
        }
        drop(ss);

        if !Path::new(&self.path).exists() {
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
        let path = self.path.clone();
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

    /// function run in a thread
    fn inner_monitor(shared_state: Arc<Mutex<SharedState>>, path: &str) -> Result<()> {
        let ss_clone = Arc::clone(&shared_state);
        Self::set_panic_hook(ss_clone);

        smol::block_on(async {
            let (tx, rx) = mpsc::channel::<Result<NotifyEvent>>();
            let mut watcher = notify::recommended_watcher(tx).unwrap();
            watcher
                .watch(Path::new(path), RecursiveMode::NonRecursive)
                .unwrap();

            let ss_clone = shared_state.clone();
            let should_stop_future = async move {
                loop {
                    let should_stop = {
                        let mut ss = ss_clone.lock().unwrap();
                        ss.elapsed_time = Utc::now().with_timezone(TIME_ZONE) - ss.launch_time;
                        ss.get_status()
                    };
                    if should_stop == Stopped {
                        break;
                    }
                    future::yield_now().await;
                }
            };

            let ss_clone2 = shared_state.clone();
            let iterate_future = async move {
                'outer: loop {
                    match rx.recv_timeout(Duration::from_millis(500)) {
                        Ok(Ok(NotifyEvent {
                            kind: EventKind::Modify(ckind),
                            paths,
                            ..
                        })) => {
                            log!(
                                ss_clone2,
                                Utc::now().with_timezone(TIME_ZONE),
                                MonitorEventType::ModifiedFile,
                                format!(
                                    "Notify event: {:?}, {:?}",
                                    EventKind::Modify(ckind),
                                    paths
                                )
                            );

                            let path = paths[0].clone();

                            // update and get old file size
                            let old_file_size = ss_clone2
                                .lock()
                                .unwrap()
                                .update_file_watchinfo(&path)
                                .unwrap_or_default()
                                .file_size;

                            let current_file_size = ss_clone2
                                .lock()
                                .unwrap()
                                .file_statistic
                                .files_watched
                                .get(&path)
                                .unwrap()
                                .file_size;

                            log!(
                                ss_clone2,
                                Utc::now().with_timezone(TIME_ZONE),
                                MonitorEventType::Info,
                                format!(
                                    "File watched updated from {} bytes to {}",
                                    old_file_size, current_file_size
                                )
                            );

                            // get file's size and last_read_pos
                            let (last_read_pos, file_size) = {
                                let ss = ss_clone2.lock().unwrap();
                                ss.file_statistic
                                    .files_watched
                                    .get(&path)
                                    .cloned()
                                    .map(|info| (info.last_read_pos, info.file_size))
                                    .unwrap_or((0, 0))
                            };

                            // if the monitor is stopped, break the loop
                            if ss_clone2.lock().unwrap().status == Stopped {
                                break 'outer;
                            }

                            // iterate the file's path strings
                            if file_size > last_read_pos {
                                let paths_stream =
                                    Box::pin(Self::extract_path_stream(&path, last_read_pos).await);

                                ss_clone2.lock().unwrap().set_files_reading(&path);
                                // collect the paths
                                let paths: Vec<(PathBuf, u64)> = paths_stream.collect().await;

                                // the offset is the file's size
                                let offset = file_size;
                                let last_offset = ss_clone2
                                    .lock()
                                    .unwrap()
                                    .set_file_watchinfo(
                                        &path,
                                        FileWatchInfo {
                                            last_read_pos: offset,
                                            file_size,
                                        },
                                    )
                                    .unwrap_or(FileWatchInfo {
                                        last_read_pos: 0,
                                        file_size: 0,
                                    })
                                    .last_read_pos;

                                let bytes_read = offset - last_offset;

                                log!(
                                    ss_clone2,
                                    Utc::now().with_timezone(TIME_ZONE),
                                    MonitorEventType::ModifiedFile,
                                    format!("Read {} bytes from file {:?}", bytes_read, path)
                                );

                                ss_clone2.lock().unwrap().add_file_got(paths.len());
                            }
                        }
                        Ok(_) => {}
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(e) => {
                            log!(
                                ss_clone2,
                                Utc::now().with_timezone(TIME_ZONE),
                                MonitorEventType::Error,
                                format!("Error: {:?}", e)
                            );
                            break;
                        }
                    }
                }
            };

            futures::join!(should_stop_future, iterate_future);

            log!(
                shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                MonitorEventType::Info,
                "Monitor stopped".to_string()
            );

            drop(watcher);
        });
        Ok(())
    }

    async fn extract_path_stream(
        path: &PathBuf,
        offset: u64,
    ) -> impl stream::Stream<Item = (PathBuf, u64)> + '_ {
        let file = fs::File::open(path).await.unwrap();
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::Start(offset)).await.unwrap();

        stream::unfold(
            (reader, offset),
            move |(mut reader, mut current_offset)| async move {
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line).await {
                        Ok(0) => return None, // EOF
                        Ok(n) => {
                            let new_offset = current_offset + n as u64;
                            let words = line.split_whitespace().collect::<Vec<&str>>();
                            if words.len() == 6 && words[3] == "STOR" && words[4] == "226" {
                                let path_str =
                                    line.split(words[4]).collect::<Vec<&str>>()[1].trim();
                                return Some((
                                    (PathBuf::from(path_str), new_offset),
                                    (reader, new_offset),
                                ));
                            } else {
                                current_offset = new_offset;
                                continue;
                            }
                        }
                        Err(e) => {
                            eprintln!("Error reading log line: {}", e);
                            return None;
                        }
                    }
                }
            },
        )
    }

    pub fn set_lunch_time(&self) {
        self.shared_state.lock().unwrap().launch_time = Utc::now().with_timezone(TIME_ZONE);
    }

    pub fn get_lunch_time(&self) -> String {
        self.shared_state
            .lock()
            .unwrap()
            .launch_time
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
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

    pub fn set_status(&self, status: MonitorStatus) {
        self.shared_state.lock().unwrap().set_status(status);
    }

    pub fn get_status(&self) -> MonitorStatus {
        self.shared_state.lock().unwrap().get_status()
    }

    pub fn files_got(&self) -> usize {
        self.shared_state.lock().unwrap().file_statistic.files_got
    }

    pub fn file_reading(&self) -> PathBuf {
        self.shared_state
            .lock()
            .unwrap()
            .file_statistic
            .file_reading
            .clone()
    }

    pub fn files_recorded(&self) -> usize {
        self.shared_state
            .lock()
            .unwrap()
            .file_statistic
            .files_recorded
    }

    fn set_panic_hook(shared_state: Arc<Mutex<SharedState>>) {
        panic::set_hook(Box::new(move |panic_info| {
            log!(
                shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                MonitorEventType::Error,
                format!("Thread panicked: {:?}", panic_info)
            );
            let mut ss = shared_state.lock().unwrap();
            ss.status = Stopped;
        }));
    }
}

impl SharedState {
    fn add_logs(&mut self, event: MonitorEvent) {
        self.logs.add_raw_item(event);
    }

    /// Set or init watch file's `FileStatistics` if not exist, and return the old value.
    fn update_file_watchinfo(&mut self, path: &PathBuf) -> Option<FileWatchInfo> {
        let file_size = std::fs::metadata(path).unwrap().len();

        let file_watch_info = if let Some(info) = self.file_statistic.files_watched.get(path) {
            FileWatchInfo {
                last_read_pos: info.last_read_pos,
                file_size,
            }
        } else {
            FileWatchInfo {
                last_read_pos: 0,
                file_size,
            }
        };

        self.file_statistic
            .files_watched
            .insert(path.clone(), file_watch_info.clone())
    }

    fn set_file_watchinfo(&mut self, path: &PathBuf, info: FileWatchInfo) -> Option<FileWatchInfo> {
        self.file_statistic.files_watched.insert(path.clone(), info)
    }

    fn add_file_got(&mut self, num: usize) {
        self.file_statistic.files_got += num;
    }

    fn get_status(&self) -> MonitorStatus {
        self.status.clone()
    }

    fn set_status(&mut self, status: MonitorStatus) {
        self.status = status;
    }

    fn set_files_reading(&mut self, path: &PathBuf) {
        self.file_statistic.file_reading = path.clone();
    }
}

#[macro_export]
macro_rules! log {
    ($shared_state:expr, $time:expr, $event_type:expr, $message:expr $(,)* ) => {
        $shared_state.lock().unwrap().add_logs(MonitorEvent {
            time: Some($time),
            event_type: $event_type,
            message: $message,
        })
    };
}
