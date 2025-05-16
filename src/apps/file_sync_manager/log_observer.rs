use std::{
    panic,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use indexmap::IndexMap;

use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use futures;
use notify::{Event as NotifyEvent, EventKind, RecursiveMode, Result, Watcher};
use smol::{
    fs,
    future::{self},
    io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom},
    pin,
    stream::{self, StreamExt},
};

use crate::{
    EK::*,
    LOE::*,
    OneEvent,
    ProgressStatus::{self, *},
    TIME_ZONE,
    apps::file_sync_manager::registry,
    load_config,
    my_widgets::wrap_list::WrapList,
};

macro_rules! log {
    ($shared_state:expr, $time:expr, $kind:expr, $content:expr $(,)* ) => {
        $shared_state.lock().unwrap().add_logs(OneEvent {
            time: Some($time),
            kind: $kind,
            content: $content,
        })
    };
}
pub struct LogObserver {
    pub path: PathBuf,
    pub shared_state: Arc<Mutex<SharedState>>,
    pub handle: Option<thread::JoinHandle<Result<()>>>,
}

pub struct SharedState {
    pub launch_time: DateTime<FixedOffset>,
    pub elapsed_time: TimeDelta,
    pub status: ProgressStatus,
    pub file_statistic: FileStatistics,
    pub logs: WrapList,
}

#[derive(Default)]
pub struct FileStatistics {
    files_watched: IndexMap<PathBuf, FileWatchInfo>,
    files_got: usize,
    files_recorded: usize,
    file_reading: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FileWatchInfo {
    last_read_pos: u64,
    file_size: u64,
}

impl LogObserver {
    pub fn new(path: PathBuf, log_size: usize) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            launch_time: DateTime::from_timestamp(0, 0)
                .unwrap()
                .with_timezone(TIME_ZONE),
            elapsed_time: TimeDelta::zero(),
            status: Stopped,
            file_statistic: FileStatistics::default(),
            logs: WrapList::new(log_size),
        }));

        LogObserver {
            path,
            shared_state,
            handle: None,
        }
    }

    pub fn stop_observer(&mut self) {
        let status = self.shared_state.lock().unwrap().status;
        if status == Stopped || status == Stopping {
            log!(
                self.shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                LogObserverEvent(Error),
                "Observer is already stopped or stopping.".to_string()
            );
            return;
        }

        self.shared_state.lock().unwrap().set_status(Stopped);

        let ss_clone = self.shared_state.clone();

        if let Some(handle) = self.handle.take() {
            let future = async move {
                loop {
                    if handle.is_finished() {
                        ss_clone.lock().unwrap().reset_time();
                        log!(
                            ss_clone,
                            Utc::now().with_timezone(TIME_ZONE),
                            LogObserverEvent(Stop),
                            "Observer is stopping.".to_string()
                        );
                    } else {
                        log!(
                            ss_clone,
                            Utc::now().with_timezone(TIME_ZONE),
                            LogObserverEvent(Error),
                            "Observer doesn't stop.".to_string()
                        );
                    }
                    future::yield_now().await;
                }
            };

            smol::spawn(future).detach();
        }
    }

    pub fn start_observer(&mut self) -> Result<()> {
        if !Path::new(&self.path).exists() {
            let current_path = std::env::current_dir()?;
            log!(
                self.shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                LogObserverEvent(Error),
                format!(
                    "Start failed: path does not exist, current path: {}, please configure the path parameter in cfg.json ",
                    current_path.display()
                )
            );
            return Ok(());
        }

        let status = self.shared_state.lock().unwrap().status;
        match status {
            Running(_) | Stopping => {
                log!(
                    self.shared_state,
                    Utc::now().with_timezone(TIME_ZONE),
                    LogObserverEvent(Error),
                    "Observer is running or stopping.".to_string()
                );
                return Ok(());
            }
            _ => {}
        }

        self.set_launch_time();
        self.set_status(Running(crate::Running::Periodic));

        let time = Utc::now().with_timezone(TIME_ZONE);
        self.shared_state.lock().unwrap().launch_time = time;

        let cloned_shared_state = Arc::clone(&self.shared_state);
        let path = self.path.clone();
        let handle =
            thread::spawn(move || LogObserver::inner_observer(cloned_shared_state, path, None));

        self.handle = Some(handle);

        log!(
            self.shared_state,
            Utc::now().with_timezone(TIME_ZONE),
            LogObserverEvent(Start),
            "Observer started".to_string()
        );
        Ok(())
    }

    // 线程中运行
    fn inner_observer(
        shared_state: Arc<Mutex<SharedState>>,
        path: PathBuf,
        poll_duration: Option<Duration>,
    ) -> Result<()> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (tx, rx) = mpsc::channel::<Result<NotifyEvent>>();
            let mut watcher = notify::recommended_watcher(tx).unwrap();
            // 设为轮询模式
            if let Some(duration) = poll_duration {
                watcher
                    .configure(notify::Config::default().with_poll_interval(duration))
                    .unwrap();
            }
            watcher.watch(&path, RecursiveMode::NonRecursive).unwrap();

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
                let max_files_watched = load_config().file_sync_manager.max_observed_files;
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
                                LogObserverEvent(ModifiedFile),
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
                                .update_file_watchinfo(&path, max_files_watched)
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
                                LogObserverEvent(Info),
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

                            // if the Observer is stopped, break the loop
                            if ss_clone2.lock().unwrap().status == Stopped {
                                break 'outer;
                            }

                            // iterate the file's path strings
                            if file_size > last_read_pos {
                                let paths_stream =
                                    Box::pin(Self::extract_path_stream(&path, last_read_pos).await);

                                ss_clone2.lock().unwrap().set_files_reading(&path);
                                // collect the paths
                                let paths_and_offset: Vec<(PathBuf, u64)> =
                                    paths_stream.collect().await;

                                let paths: Vec<PathBuf> =
                                    paths_and_offset.iter().map(|f| f.0.clone()).collect();
                                registry::process_paths(paths).await.unwrap();

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
                                    LogObserverEvent(Info),
                                    format!("Read {} bytes from file {:?}", bytes_read, path)
                                );

                                ss_clone2
                                    .lock()
                                    .unwrap()
                                    .add_file_got(paths_and_offset.len());
                            }
                        }
                        Ok(_) => {}
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(e) => {
                            log!(
                                ss_clone2,
                                Utc::now().with_timezone(TIME_ZONE),
                                LogObserverEvent(Error),
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
                LogObserverEvent(Stop),
                "Observer stopped".to_string()
            );

            drop(watcher);
        });
        Ok(())
    }

    // 读取指定路径中从指定偏移量开始的内容，并提取FTP接收的文件路径
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

                            if let Some(words) = line.split_once("STOR 226 ") {
                                let path_str = words.1.trim_end();
                                return Some((
                                    (Self::handle_pathstring(path_str).await, new_offset),
                                    (reader, new_offset),
                                ));
                            }
                            current_offset = new_offset;
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

    async fn handle_pathstring(path: &str) -> PathBuf {
        // 转换为windows风格
        let path = path.replace('/', r#"\"#).replace('+', " ");

        // 读取配置
        let prefix_map = load_config().file_sync_manager.prefix_map_of_extract_path;

        // 遍历所有映射，优先非"default"
        for (_key, pair) in prefix_map.iter().filter(|(k, _)| *k != "default") {
            let (from, to) = (&pair[0], &pair[1]);
            if path.starts_with(from) && !from.is_empty() {
                let replaced = format!("{}{}", to, path.trim_start_matches(from));
                return PathBuf::from(replaced);
            }
        }
        // 没有匹配到则用"default"
        if let Some(pair) = prefix_map.get("default") {
            let (from, to) = (&pair[0], &pair[1]);
            let replaced = format!("{}{}", to, path.trim_start_matches(from));
            return PathBuf::from(replaced);
        }
        // 没有default则原样返回
        PathBuf::from(path)
    }

    pub fn set_launch_time(&self) {
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
        ss.reset_time();
    }

    pub fn set_status(&self, status: ProgressStatus) {
        self.shared_state.lock().unwrap().set_status(status);
    }

    pub fn get_status(&self) -> ProgressStatus {
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

    pub fn get_logs_str(&self) -> Vec<String> {
        let logs = &self.shared_state.lock().unwrap().logs;
        logs.get_raw_list_string()
    }

    pub fn get_logs_item(&self) -> Vec<OneEvent> {
        self.shared_state.lock().unwrap().logs.get_raw_list().into()
    }

    pub fn get_logs_widget(&self) -> WrapList {
        self.shared_state.lock().unwrap().logs.clone()
    }

    fn set_panic_hook(shared_state: Arc<Mutex<SharedState>>) {
        panic::set_hook(Box::new(move |panic_info| {
            log!(
                shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                LogObserverEvent(Error),
                format!("Thread panicked: {:?}", panic_info)
            );
            let mut ss = shared_state.lock().unwrap();
            ss.status = Stopped;
        }));
    }
}

impl SharedState {
    fn add_logs(&mut self, event: OneEvent) {
        self.logs.add_raw_item(event);
    }

    /// Set or init watch file's `FileStatistics` if not exist, and return the old value.
    fn update_file_watchinfo(
        &mut self,
        path: &PathBuf,
        max_files_watched: usize,
    ) -> Option<FileWatchInfo> {
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

        // 插入前检查容量，超出则移除最早的
        if !self.file_statistic.files_watched.contains_key(path)
            && self.file_statistic.files_watched.len() >= max_files_watched
        {
            // 移除最早插入的项
            self.file_statistic.files_watched.shift_remove_index(0);
        }

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

    fn get_status(&self) -> ProgressStatus {
        self.status.clone()
    }

    fn set_status(&mut self, status: ProgressStatus) {
        self.status = status;
    }

    fn set_files_reading(&mut self, path: &PathBuf) {
        self.file_statistic.file_reading = path.clone();
    }

    fn reset_time(&mut self) {
        self.launch_time = DateTime::from_timestamp(0, 0)
            .unwrap()
            .with_timezone(TIME_ZONE);
        self.elapsed_time = TimeDelta::zero();
    }
}

#[tokio::test]
async fn test_path_construction() {
    let path = LogObserver::handle_pathstring("/CTA8280H/TEST-48/DA35_BP85226D_P01DB_TP16D252_250417237_BP85226_P01DB9X_HDJJ13D._PL_20250507_141512.CAT").await;

    let path_ac03 = LogObserver::handle_pathstring("/AC03/ASDFDSAFDSA.csv").await;

    let path_with_whitespace = LogObserver::handle_pathstring("/OS2000/AS  DFDSAFDSA.csv").await;

    // windows iis ftp日志会将路径中间的空格替换为`+`号，将`+`不做处理
    let path_with_special_char = LogObserver::handle_pathstring("/123/++Starting+Space/Mix!@#$%^&()=+{}[];',~_目录/Sub+Folder+中间+空+格/文件_🌟Unicode_引号_&_Sp++ecial_Chars_最终版_v2.0%20@2024").await;

    assert_eq!(
        PathBuf::from("E:\\CusData\\AC03\\ASDFDSAFDSA.csv"),
        path_ac03
    );
    assert_eq!(
        PathBuf::from(
            "E:\\testdata\\CTA8280H\\TEST-48\\DA35_BP85226D_P01DB_TP16D252_250417237_BP85226_P01DB9X_HDJJ13D._PL_20250507_141512.CAT"
        ),
        path
    );
    assert_eq!(
        PathBuf::from("E:\\testdata\\OS2000\\AS  DFDSAFDSA.csv"),
        path_with_whitespace
    );
    assert_eq!(
        PathBuf::from(
            "E:\\testdata\\123\\  Starting Space\\Mix!@#$%^&()= {}[];',~_目录\\Sub Folder 中间 空 格\\文件_🌟Unicode_引号_&_Sp  ecial_Chars_最终版_v2.0%20@2024"
        ),
        path_with_special_char
    );
}

#[test]
fn test_file_path() {
    let path = PathBuf::from("asset\\cfg.json");
    if !std::fs::exists(&path).unwrap() {
        eprintln!(
            "File does not exist, current path is {}, and your path is {}",
            std::env::current_dir().unwrap().display(),
            path.display()
        );
        panic!();
    }
}

#[tokio::test]
async fn test_extract_path() {
    assert_eq!(
    extract_path(
        "2025-05-07 16:42:15 10.53.2.70 STOR 226 /CTA8280H/TEST-48/DA35_BP85226D_P01DB_TP16D252_250417237_BP85226_P01DB9X_HDJJ13D._PL_20250507_141512.CAT").await,
        PathBuf::from(
            "E:\\testdata\\CTA8280H\\TEST-48\\DA35_BP85226D_P01DB_TP16D252_250417237_BP85226_P01DB9X_HDJJ13D._PL_20250507_141512.CAT"
            ),
    );
    assert_eq!(
        extract_path("2025-05-07 16:42:15 10.53.2.70 STOR 226 /OS2000/AS DFDSAFDSA.csv").await,
        PathBuf::from("E:\\testdata\\OS2000\\AS DFDSAFDSA.csv"),
    );
}

async fn extract_path(content: &str) -> PathBuf {
    let base = std::env::temp_dir().join("test_assdfasset");
    std::fs::create_dir_all(&base).unwrap();
    let file = base.join("fileasdfsfsadfasd");
    std::fs::write(&file, content).unwrap();

    let extracted_paths = LogObserver::extract_path_stream(&file, 0).await;
    pin!(extracted_paths);

    let path = extracted_paths.next().await.unwrap();
    std::fs::remove_dir_all(&base).unwrap();
    path.0
}
