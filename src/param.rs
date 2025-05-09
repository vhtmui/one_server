use crate::{cli, get_param};

pub const PARAM_HELP: &str = "help";
pub const PARAM_CONFIG_PATH: &str = "cfg=";
pub const PARAM_CLI: &str = "cli";


pub fn handle_params() {
    if let Some(_) = get_param(PARAM_HELP) {
        print_params_help();
    }
    if let Some(_) = get_param(PARAM_CLI) {
        cli::run_cli_mode();
        return;
    }
}

pub fn default_config_path() -> String {
    if cfg!(debug_assertions) {
        "asset/cfg.json".to_string()
    } else {
        "/etc/one_server/cfg.json".to_string()
    }
}

fn print_params_help() {
    println!("参数列表：");
    println!("  --help                   显示帮助信息");
    println!("  --cfg=<path>             指定配置文件路径");
    println!("  --cli                    cli模式");
}
