use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use chrono::{DateTime, FixedOffset, Utc};
use walkdir::{DirEntry, WalkDir};

use crate::{
    DirScannerEventKind::*,
    EK::*,
    OneEvent,
    ProgressStatus::{self, *},
    Running, TIME_ZONE,
    apps::file_sync_manager::registry,
    my_widgets::wrap_list::WrapList,
};

macro_rules! log {
    ($shared_state:expr,  $kind:expr, $content:expr $(,)* ) => {
        $shared_state.lock().unwrap().add_logs(OneEvent {
            time: Some(Utc::now().with_timezone(TIME_ZONE)),
            kind: DirScannerEvent($kind),
            content: $content,
        })
    };
}

pub struct DirScanner {
    pub shared_state: Arc<Mutex<ScSharedState>>,
    path: PathBuf,
}

pub struct ScSharedState {
    pub logs: WrapList,
    pub scanner_status: ProgressStatus,
    periodic_scan_count: usize,
}

impl DirScanner {
    pub fn new(log_size: usize) -> Self {
        Self {
            shared_state: Arc::new(Mutex::new(ScSharedState {
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
        let ss_clone = self.shared_state.clone();

        let path = self.path.clone();
        if !path.exists() {
            let msg = format!("Path does not exist: {}", path.display());
            log!(ss_clone, Error, msg);
            return Ok(());
        }

        let status = ss_clone.lock().unwrap().scanner_status.clone();
        match status {
            Running(_) => {
                log!(ss_clone, Error, "Scanner already running".to_string());
                return Ok(());
            }
            Stopping => {
                log!(ss_clone, Error, "Scanner is stopping".to_string());
                return Ok(());
            }
            _ => {
                ss_clone.lock().unwrap().set_status(Running(Running::Once));
            }
        }

        let ss_clone2 = ss_clone.clone();
        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                Self::collect_and_update_fileinfo(ss_clone2, &path, |e| e.file_type().is_file())
                    .await?;
                Ok::<(), std::io::Error>(())
            })?;
            Ok::<(), std::io::Error>(())
        });

        log!(ss_clone, Start, "Scanner started".to_string());

        let future = async move {
            loop {
                let msg = format!("handle status: {:?}", handle.is_finished());
                log!(ss_clone, Info, msg);

                if handle.is_finished() {
                    log!(ss_clone, Info, "Handler finished".to_string());

                    ss_clone.lock().unwrap().set_status(Finished);
                    let handle_result = handle.join().unwrap();

                    let msg = format!("Scanner completed with result {:?}", handle_result);
                    log!(ss_clone, Complete, msg);

                    break;
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        };

        tokio::spawn(future);
        Ok(())
    }

    pub fn start_periodic_scan(&self, interval: Duration) {
        let ss_clone = self.shared_state.clone();

        if std::fs::metadata(&self.path).is_err() {
            let msg = format!("Path does not exist: {}", self.path.display());
            log!(ss_clone, Error, msg);
            return;
        }

        let status = ss_clone.lock().unwrap().scanner_status.clone();
        if let Running(_) = status {
            log!(ss_clone, Error, "Scanner already running".to_string());
            return;
        }

        ss_clone
            .lock()
            .unwrap()
            .set_status(Running(Running::Periodic));

        let path = self.path.clone();
        let _ = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                'out: loop {
                    let now = Utc::now().with_timezone(TIME_ZONE);
                    let cutoff_time = now - interval;

                    let status = ss_clone.lock().unwrap().scanner_status.clone();
                    if let Running(Running::Periodic) = status {
                        let scan_count = ss_clone.lock().unwrap().add_scan_count();
                        let msg = format!("Start periodic scan, count {}.", scan_count);
                        log!(ss_clone, Start, msg);

                        let _ =
                            DirScanner::collect_and_update_fileinfo(ss_clone.clone(), &path, |e| {
                                e.file_type().is_file()
                                    && match e.metadata() {
                                        Ok(meta) => {
                                            let modified: DateTime<FixedOffset> = meta
                                                .modified()
                                                .map(|t| {
                                                    DateTime::<Utc>::from(t)
                                                        .with_timezone(TIME_ZONE)
                                                })
                                                .unwrap();
                                            modified >= cutoff_time
                                        }
                                        Err(_) => false,
                                    }
                            })
                            .await;

                        let msg = format!("Periodic scan completed, count {}", scan_count);
                        log!(ss_clone, Complete, msg);

                        let sleep_step = std::time::Duration::from_secs(1);
                        let mut slept = std::time::Duration::ZERO;
                        while slept < interval {
                            tokio::time::sleep(sleep_step).await;

                            slept += sleep_step;
                            let status = ss_clone.lock().unwrap().scanner_status.clone();
                            if status != Running(Running::Periodic) {
                                ss_clone.lock().unwrap().set_status(Stopped);
                                log!(
                                    ss_clone,
                                    Stop,
                                    "Periodic scanner stopped manually".to_string()
                                );

                                break 'out;
                            }
                        }
                    } else {
                        ss_clone.lock().unwrap().set_status(Stopped);
                        log!(
                            ss_clone,
                            Stop,
                            "Periodic scanner stopped manually".to_string()
                        );
                        break;
                    }
                }
            });
        });
    }

    pub fn stop_periodic_scan(&self) {
        let status = self.shared_state.lock().unwrap().scanner_status.clone();

        if status == Stopped || status == Stopping {
            log!(
                self.shared_state,
                Error,
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
                    log!(ss_clone, Stop, "Scanner stopped".to_string());
                    break;
                }
                tokio::task::yield_now().await;
            }
        };

        tokio::spawn(future);
    }

    async fn collect_and_update_fileinfo<F>(
        shared_state: Arc<Mutex<ScSharedState>>,
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

        let msg = format!(
            "Found {} files in the directory: {}",
            files.len(),
            dir.display()
        );
        log!(shared_state, Info, msg);

        // 调用数据库更新
        registry::update_file_infos_to_db(files).await?;

        log!(shared_state, DBInfo, "DB update finished.".to_string());
        Ok(())
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

impl ScSharedState {
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
