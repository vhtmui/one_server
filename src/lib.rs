pub mod apps;
pub mod my_widgets;
pub mod cli;

use serde::Deserialize;
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Deserialize)]
pub struct FileMonitorConfig {
    pub prefix_map_of_extract_path: HashMap<String, [String; 2]>,
    pub monitor_path: PathBuf,
}

#[derive(Deserialize)]
pub struct Config {
    pub file_monitor: FileMonitorConfig,
}

#[test]
fn test_config() {
    let config_str = fs::read_to_string("asset/cfg.json").unwrap();
    let _config: Config = serde_json::from_str(&config_str).unwrap();
}