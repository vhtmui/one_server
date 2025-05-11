pub mod apps;
pub mod cli;
pub mod my_widgets;
pub mod param;

use chrono::{DateTime, FixedOffset};
use param::default_config_path;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Deserialize)]
pub struct FileMonitorConfig {
    pub prefix_map_of_extract_path: HashMap<String, [String; 2]>,
    pub monitor_path: PathBuf,
    pub max_watch_files: usize,
}

#[derive(Deserialize)]
pub struct MyConfig {
    pub file_monitor: FileMonitorConfig,
}

pub fn load_config() -> MyConfig {
    let path = get_param(param::PARAM_CONFIG_PATH);

    let config_str = fs::read_to_string(path.unwrap_or_else(|| default_config_path())).unwrap();
    let config: MyConfig = serde_json::from_str(&config_str).unwrap();
    config
}

pub fn get_param(param: &str) -> Option<String> {
    let args = std::env::args();
    if param.ends_with('=') {
        // 赋值参数，形如 "cfg="
        let prefix = format!("--{}", param);
        for arg in args {
            if arg.starts_with(&prefix) {
                let value = arg[prefix.len()..]
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                return Some(value);
            }
        }
        None
    } else {
        // 开关参数，形如 "cli"
        let flag = format!("--{}", param);
        for arg in args {
            if arg == flag {
                return Some("".to_string());
            }
        }
        None
    }
}

pub struct Event {
    kind: EventKind,
    content: String,
    time: Option<DateTime<FixedOffset>>,
}

pub enum EventKind {
    LogObserverEvent(LogObserverEvent),
    DirScannerEvent(DirScannerEvent),
}

pub enum LogObserverEvent {
    Stop,
    Error,
    CreatedFile,
    ModifiedFile,
    DeletedFile,
    Info,
}

pub enum DirScannerEvent {
    Start,
    Stop,
    Complete,
    Error,
    Info,
}

#[test]
fn validate_config() {
    let config_str = fs::read_to_string("asset/cfg.json").unwrap();
    let _config: MyConfig = serde_json::from_str(&config_str).unwrap();
}
