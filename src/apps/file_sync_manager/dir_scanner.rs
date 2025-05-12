use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use walkdir::WalkDir;

use crate::{apps::file_sync_manager::registry, my_widgets::wrap_list::WrapList};
use DirScannerStatus::*;

pub struct DirScanner {
    shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
    logs: WrapList,
    pub scanner_status: DirScannerStatus,
}

#[derive(PartialEq, Eq)]
enum DirScannerStatus {
    Running,
}

impl DirScanner {
    pub fn new() -> Self {
        Self {
            shared_state: Arc::new(Mutex::new(SharedState {
                logs: WrapList::new(100),
                scanner_status: DirScannerStatus::Stopped,
            })),
        }
    }

    pub fn start_scanner(&mut self, path: PathBuf) -> std::io::Result<()> {
        if self.shared_state.lock().unwrap().scanner_status == Running {
            log!(
                self.shared_state,
                Utc::now().with_timezone(TIME_ZONE),
                MonitorEventType::Scanner,
                "Scanner already running".to_string()
            );
            return Ok(());
        }

        shared_state.lock().unwrap().set_scanner_status(Running);

        let ss_clone2 = shared_state.clone();
        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                LogObserver::scan_and_update_dir(ss_clone2, &path).await?;
                Ok::<(), std::io::Error>(())
            })?;
            Ok::<(), std::io::Error>(())
        });

        log!(
            shared_state,
            Utc::now().with_timezone(TIME_ZONE),
            MonitorEventType::Scanner,
            "Scanner started".to_string()
        );

        let future = async move {
            loop {
                if handle.is_finished() {
                    shared_state.lock().unwrap().set_scanner_status(Finished);
                    let handle_result = handle.join().unwrap();

                    log!(
                        shared_state,
                        Utc::now().with_timezone(TIME_ZONE),
                        MonitorEventType::Scanner,
                        format!("Scanner finished with {:?}", handle_result)
                    );
                    break;
                }

                smol::future::yield_now().await;
            }
        };

        smol::spawn(future).detach();
        Ok(())
    }

    pub async fn scan_and_update_dir(
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
            MonitorEventType::Scanner,
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
            MonitorEventType::Scanner,
            "DB update finished.".to_string()
        );
        Ok(())
    }
}
