use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use chrono::Utc;
use walkdir::WalkDir;

use crate::{
    DSE::*,
    EK::*,
    OneEvent,
    ProgressStatus::{self, *},
    TIME_ZONE,
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
    shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
    logs: WrapList,
    pub scanner_status: ProgressStatus,
}

impl DirScanner {
    pub fn new(log_size: usize) -> Self {
        Self {
            shared_state: Arc::new(Mutex::new(SharedState {
                logs: WrapList::new(log_size),
                scanner_status: Stopped,
            })),
        }
    }

    pub fn start_scanner(&mut self, path: PathBuf) -> std::io::Result<()> {
        if !path.exists() {
            log!(
                self.shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                DirScannerEvent(Error),
                format!("Path does not exist: {}", path.display())
            );
            return Ok(());
        }

        if self.shared_state.lock().unwrap().scanner_status == Running {
            log!(
                self.shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                DirScannerEvent(Error),
                "Scanner already running".to_string()
            );
            return Ok(());
        }

        let shared_state = self.shared_state.clone();

        shared_state.lock().unwrap().set_status(Running);

        let ss_clone2 = shared_state.clone();
        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                Self::scan_and_update_dir(ss_clone2, &path).await?;
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
                        format!("Scanner completed with {:?}", handle_result)
                    );
                    break;
                }

                smol::future::yield_now().await;
            }
        };

        smol::spawn(future).detach();
        Ok(())
    }

    async fn scan_and_update_dir(
        shared_state: Arc<Mutex<SharedState>>,
        dir: &Path,
    ) -> std::io::Result<()> {
        // 递归收集所有文件路径
        let files: Vec<PathBuf> = WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
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

    pub fn start_periodic_scan(&self, path: PathBuf, interval: std::time::Duration) {
        let mut scanner = self.clone();
        smol::spawn(async move {
            loop {
                let _ = scanner.start_scanner(path.clone());
                smol::Timer::after(interval).await;
            }
        })
        .detach();
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

    pub fn get_logs_widget(&self) -> WrapList {
        self.shared_state.lock().unwrap().logs.clone()
    }
}

impl SharedState {
    fn add_logs(&mut self, event: OneEvent) {
        self.logs.add_raw_item(event);
    }

    fn set_status(&mut self, status: ProgressStatus) {
        self.scanner_status = status;
    }
}

impl Clone for DirScanner {
    fn clone(&self) -> Self {
        Self {
            shared_state: self.shared_state.clone(),
        }
    }
}
