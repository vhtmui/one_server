use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::PathBuf,
};

use crate::{Config, apps::file_monitor::FileMonitor};

// 命令常量定义
pub const CMD_QUIT: &str = ":q";
pub const CMD_HELP: &str = "help";
pub const CMD_START_MONITOR: &str = "stamo";
pub const CMD_STOP_MONITOR: &str = "stomo";
pub const CMD_INTO_SCANNER: &str = "intosc";
pub const CMD_SHOW_STATUS: &str = "showstatus";
pub const CMD_SHOW_LOGS: &str = "showlogs";
pub const CMD_START_SCAN: &str = "start";
pub const CMD_INPUT_DIR: &str = "<Dir>";

pub fn run_cli_mode() {
    println!("进入命令行模式，输入 help 查看命令，:q 退出。");
    let mut input = String::new();
    loop {
        print!("\\> ");
        io::stdout().flush().unwrap();
        input.clear();
        if io::stdin().read_line(&mut input).is_err() {
            println!("读取输入失败");
            continue;
        }
        let cmd = input.trim();
        match cmd {
            CMD_QUIT => break,
            CMD_HELP => {
                help("main");
            }
            CMD_START_MONITOR => {
                // 调用你的监控启动逻辑
                println!("监控已启动");
            }
            CMD_STOP_MONITOR => {
                // 调用你的监控停止逻辑
                println!("监控已停止");
            }
            CMD_INTO_SCANNER => {
                into_scanner();
            }
            "" => {}
            _ => println!("未知命令，输入 help 查看帮助"),
        }
    }
    println!("已退出命令行模式。");
}

fn into_scanner() {
    let path: Config =
        serde_json::from_str(&fs::read_to_string("asset\\cfg.json").unwrap()).unwrap();
    let mut file_monitor = FileMonitor::new(
        "file_monitor".to_string(),
        path.file_monitor.monitor_path,
        50,
    );
    print!("进入目录扫描模式：");
    loop {
        print!("\\scaner> ");
        io::stdout().flush().unwrap();
        let mut cmd = String::new();
        if io::stdin().read_line(&mut cmd).is_err() {
            println!("读取输入失败");
            continue;
        }
        let cmd = cmd.trim();
        match cmd {
            CMD_QUIT => break,
            CMD_HELP => {
                help("scanner");
            }
            CMD_SHOW_STATUS => {
                println!("监控器状态：{:?}", file_monitor.monitor.get_status());
                println!(
                    "扫描器状态：{:?}",
                    file_monitor.monitor.get_scanner_status()
                );
            }
            CMD_SHOW_LOGS => {
                println!("日志：");
                for log in file_monitor.monitor.get_logs() {
                    println!("{}", log);
                }
            }
            CMD_START_SCAN => {
                println!("输入目录路径：");
                loop {
                    print!("scanner\\inputdir> ");
                    io::stdout().flush().unwrap();
                    let mut cmd = String::new();
                    if io::stdin().read_line(&mut cmd).is_err() {
                        println!("  读取输入失败");
                        continue;
                    }
                    let dir = cmd.trim();
                    if dir.is_empty() {
                        println!("  输入为空，请重新输入");
                        continue;
                    }
                    if fs::metadata(dir).is_ok() {
                        file_monitor
                            .monitor
                            .start_scanner(PathBuf::from(dir))
                            .unwrap();
                        println!("开始扫描目录：{}", dir);
                        break;
                    } else {
                        println!("目录不存在，请重新输入");
                    }
                }
            }
            "" => {}
            _ => {}
        }
    }
}

fn help(topic: &str) {
    // 命令及描述列表
    let helps = HashMap::from([
        // MARK: main
        (CMD_START_MONITOR, (CMD_START_MONITOR, "启动文件监控")),
        (CMD_STOP_MONITOR, (CMD_STOP_MONITOR, "停止文件监控")),
        (CMD_INTO_SCANNER, (CMD_INTO_SCANNER, "进入扫描模式")),
        (CMD_HELP, (CMD_HELP, "查看帮助")),
        (CMD_QUIT, (CMD_QUIT, "退出")),
        // MARK: scanner
        (CMD_INPUT_DIR, (CMD_INPUT_DIR, "输入目录路径")),
        (CMD_SHOW_STATUS, (CMD_SHOW_STATUS, "查看状态")),
        (CMD_SHOW_LOGS, (CMD_SHOW_LOGS, "查看日志")),
        (CMD_START_SCAN, (CMD_START_SCAN, "开始扫描")),
    ]);
    println!("命令列表：");

    let mut output_cmds: Vec<(&str, &str)> = Vec::new();
    match topic {
        "main" => {
            for key in [
                CMD_START_MONITOR,
                CMD_STOP_MONITOR,
                CMD_INTO_SCANNER,
                CMD_HELP,
                CMD_QUIT,
            ] {
                if let Some(&(cmd, desc)) = helps.get(key) {
                    output_cmds.push((cmd, desc));
                }
            }
        }
        "scanner" => {
            for key in [
                CMD_QUIT,
                CMD_HELP,
                CMD_SHOW_STATUS,
                CMD_SHOW_LOGS,
                CMD_START_SCAN,
            ] {
                if let Some(&(cmd, desc)) = helps.get(key) {
                    output_cmds.push((cmd, desc));
                }
            }
        }
        _ => {}
    }
    // 按命令字母顺序排序
    output_cmds.sort_by(|a, b| a.0.cmp(b.0));
    for (cmd, desc) in output_cmds {
        println!("  {:<10}  {}", cmd, desc);
    }
}
