use std::{
    f32::consts::E,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use chrono::{DateTime, FixedOffset, Utc};
use walkdir::{DirEntry, WalkDir};

use crate::{
    DSE::*,
    EK::*,
    OneEvent,
    ProgressStatus::{self, *},
    Running, TIME_ZONE,
    apps::file_sync_manager::registry,
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

pub struct DirScanner {
    pub shared_state: Arc<Mutex<SharedState>>,
    path: PathBuf,
}

pub struct SharedState {
    pub logs: WrapList,
    pub scanner_status: ProgressStatus,
    periodic_scan_count: usize,
}

impl DirScanner {
    pub fn new(log_size: usize) -> Self {
        Self {
            shared_state: Arc::new(Mutex::new(SharedState {
                logs: WrapList::new(log_size),
                scanner_status: Stopped,
                periodic_scan_count: 0,
            })),
            path: PathBuf::from(""),
        }
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    pub fn start_scanner(&mut self) -> std::io::Result<()> {
        let path = self.path.clone();
        let ss_clone = self.shared_state.clone();
        if !path.exists() {
            log!(
                ss_clone,
                Utc::now().with_timezone(TIME_ZONE),
                DirScannerEvent(Error),
                format!("Path does not exist: {}", path.display())
            );
            return Ok(());
        }

        let status = ss_clone.lock().unwrap().scanner_status.clone();

        if let Running(_) = status {
            log!(
                ss_clone,
                Utc::now().with_timezone(TIME_ZONE),
                DirScannerEvent(Error),
                "Scanner already running".to_string()
            );
            return Ok(());
        }

        let shared_state = self.shared_state.clone();

        shared_state
            .lock()
            .unwrap()
            .set_status(Running(Running::Once));

        let ss_clone2 = shared_state.clone();
        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                Self::scan_and_update_dir(ss_clone2, &path, |e| e.file_type().is_file()).await?;
                Ok::<(), std::io::Error>(())
            })?;
            Ok::<(), std::io::Error>(())
        });

        log!(
            shared_state,
            Utc::now().with_timezone(TIME_ZONE),
            DirScannerEvent(Start),
            "Scanner started".to_string()
        );

        let future = async move {
            loop {
                if handle.is_finished() {
                    shared_state.lock().unwrap().set_status(Finished);
                    let handle_result = handle.join().unwrap();

                    log!(
                        shared_state,
                        Utc::now().with_timezone(TIME_ZONE),
                        DirScannerEvent(Complete),
                        format!("Scanner completed with result {:?}", handle_result)
                    );
                    break;
                }

                smol::future::yield_now().await;
            }
        };

        smol::spawn(future).detach();
        Ok(())
    }

    async fn scan_and_update_dir<F>(
        shared_state: Arc<Mutex<SharedState>>,
        dir: &Path,
        filter: F,
    ) -> std::io::Result<()>
    where
        F: Fn(&DirEntry) -> bool,
    {
        // 递归收集所有文件路径
        let files: Vec<PathBuf> = WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| filter(e))
            .map(|e| e.path().to_path_buf())
            .collect();

        log!(
            shared_state,
            Utc::now().with_timezone(TIME_ZONE),
            DirScannerEvent(Info),
            format!(
                "Found {} files in the directory. Executing insert...",
                files.len()
            )
        );

        // 调用数据库更新
        registry::process_paths(files).await?;

        log!(
            shared_state,
            Utc::now().with_timezone(TIME_ZONE),
            DirScannerEvent(DBInfo),
            "DB update finished.".to_string()
        );
        Ok(())
    }

    pub fn start_periodic_scan(&self, interval: Duration) {
        let ss_clone = self.shared_state.clone();

        let now = Utc::now().with_timezone(TIME_ZONE);
        let cutoff_time = now - interval;

        if std::fs::metadata(&self.path).is_err() {
            log!(
                ss_clone,
                Utc::now().with_timezone(TIME_ZONE),
                DirScannerEvent(Error),
                format!("Path does not exist: {}", self.path.display())
            );
            return;
        }

        let status = ss_clone.lock().unwrap().scanner_status.clone();
        if let Running(_) = status {
            log!(
                ss_clone,
                Utc::now().with_timezone(TIME_ZONE),
                DirScannerEvent(Error),
                "Scanner already running".to_string()
            );
            return;
        }

        ss_clone
            .lock()
            .unwrap()
            .set_status(Running(Running::Periodic));

        let path = self.path.clone();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.spawn(async move {
            loop {
                let status = ss_clone.lock().unwrap().scanner_status.clone();
                if let Running(Running::Periodic) = status {
                    let scan_count = ss_clone.lock().unwrap().add_scan_count();
                    log!(
                        ss_clone,
                        Utc::now().with_timezone(TIME_ZONE),
                        DirScannerEvent(Start),
                        format!("Start periodic scan, count {}.", scan_count)
                    );

                    let _ = DirScanner::scan_and_update_dir(ss_clone.clone(), &path, |e| {
                        e.file_type().is_file()
                            && match e.metadata() {
                                Ok(meta) => {
                                    let modified: DateTime<FixedOffset> = meta
                                        .modified()
                                        .map(|t| DateTime::<Utc>::from(t).with_timezone(TIME_ZONE))
                                        .unwrap();
                                    modified >= cutoff_time
                                }
                                Err(_) => false,
                            }
                    })
                    .await;
                    tokio::time::sleep(interval).await;

                    log!(
                        ss_clone,
                        Utc::now().with_timezone(TIME_ZONE),
                        DirScannerEvent(Complete),
                        format!("Periodic scan completed, count {}", scan_count)
                    );
                } else {
                    ss_clone.lock().unwrap().set_status(Stopped);
                    log!(
                        ss_clone,
                        Utc::now().with_timezone(TIME_ZONE),
                        DirScannerEvent(Stop),
                        "Periodic scanner stopped manually".to_string()
                    );
                    break;
                }
            }
        });
    }

    pub fn stop_periodic_scan(&self) {
        let status = self.shared_state.lock().unwrap().scanner_status.clone();

        if status == Stopped || status == Stopping {
            log!(
                self.shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                DirScannerEvent(Error),
                "Scanner already stopped or stopping".to_string()
            );
            return;
        }

        self.shared_state.lock().unwrap().set_status(Stopping);

        let ss_clone = self.shared_state.clone();
        let future = async move {
            loop {
                let status = ss_clone.lock().unwrap().scanner_status.clone();
                if let Stopped = status {
                    log!(
                        ss_clone,
                        Utc::now().with_timezone(TIME_ZONE),
                        DirScannerEvent(Stop),
                        "Scanner stopped".to_string()
                    );
                    break;
                }
            }
        };

        smol::spawn(future).detach();
    }

    pub fn get_status(&self) -> ProgressStatus {
        self.shared_state.lock().unwrap().scanner_status.clone()
    }

    pub fn get_logs_str(&self) -> Vec<String> {
        let logs = &self.shared_state.lock().unwrap().logs;
        logs.get_raw_list_string()
    }

    pub fn get_logs_item(&self) -> Vec<OneEvent> {
        self.shared_state.lock().unwrap().logs.get_raw_list().into()
    }

    pub fn add_logs(&mut self, event: OneEvent) {
        self.shared_state.lock().unwrap().add_logs(event);
    }
}

impl SharedState {
    fn add_logs(&mut self, event: OneEvent) {
        self.logs.add_raw_item(event);
    }

    fn set_status(&mut self, status: ProgressStatus) {
        self.scanner_status = status;
    }

    fn add_scan_count(&mut self) -> usize {
        self.periodic_scan_count += 1;
        self.periodic_scan_count
    }
}
