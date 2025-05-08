use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::PathBuf,
    vec,
};

use crate::{Config, apps::file_monitor::FileMonitor};

// 命令常量定义
pub const CMD_QUIT: &str = ":q";
pub const CMD_HELP: &str = "ls";
pub const CMD_INTO_FILEMONITOR: &str = "cd fm";
pub const CMD_START_MONITOR: &str = "start mo";
pub const CMD_STOP_MONITOR: &str = "stop mo";
pub const CMD_START_SCAN: &str = "start sc";
pub const CMD_SHOW_STATUS: &str = "ds status";
pub const CMD_SHOW_LOGS: &str = "ds logs";
pub const CMD_INPUT_DIR: &str = "<dir>";

fn read_trimmed_line(prompt: &str) -> Option<String> {
    print!("{}", prompt);
    io::stdout().flush().ok()?;
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        Some(input.trim().to_string())
    } else {
        None
    }
}

pub fn run_cli_mode() {
    println!("进入命令行模式，输入 ls 查看命令，:q 退出。");
    loop {
        let cmd = read_trimmed_line("\\> ").unwrap_or_else(|| {
            println!("读取输入失败");
            "".to_string()
        });
        match cmd.as_str() {
            CMD_QUIT => break,
            CMD_HELP => {
                help(vec![CMD_INTO_FILEMONITOR]);
            }
            CMD_INTO_FILEMONITOR => {
                into_filemonitor();
            }
            "" => {}
            _ => println!("未知命令，输入 help 查看帮助"),
        }
    }
    println!("已退出命令行模式。");
}

fn into_filemonitor() {
    // 创建文件监控器
    let path: Config =
        serde_json::from_str(&fs::read_to_string("asset\\cfg.json").unwrap()).unwrap();
    let mut file_monitor = FileMonitor::new(
        "file_monitor".to_string(),
        path.file_monitor.monitor_path,
        50,
    );
    loop {
        let cmd = read_trimmed_line("\\filemonitor> ").unwrap_or_else(|| {
            println!("读取输入失败");
            "".to_string()
        });
        match cmd.as_str() {
            CMD_QUIT => break,
            CMD_HELP => {
                help(vec![
                    CMD_SHOW_STATUS,
                    CMD_SHOW_LOGS,
                    CMD_START_SCAN,
                    CMD_QUIT,
                    CMD_HELP,
                ]);
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
                println!("  输入扫描路径：");
                loop {
                    let dir = read_trimmed_line("").unwrap_or_else(|| {
                        println!("读取输入失败");
                        "".to_string()
                    });
                    match dir.as_str() {
                        "" => {
                            println!("  输入为空，请重新输入");
                            continue;
                        }
                        CMD_QUIT => break,
                        CMD_HELP => {
                            help(vec![CMD_QUIT, CMD_HELP]);
                            continue;
                        }
                        dir => {
                            if fs::metadata(dir).is_ok() {
                                file_monitor
                                    .monitor
                                    .start_scanner(PathBuf::from(dir))
                                    .unwrap();
                                println!("开始扫描目录：{}", dir);
                                break;
                            } else {
                                print!("目录不存在，请重新输入: ");
                            }
                        }
                    }
                }
            }
            CMD_START_MONITOR => {
                println!(" 开始监控...");
                file_monitor.monitor.start_monitor().unwrap();
            }
            CMD_STOP_MONITOR => {
                println!(" 停止监控...");
                file_monitor.monitor.stop_monitor();
            }
            "" => {}
            _ => {}
        }
    }
}

fn help(cmds: Vec<&str>) {
    // 命令及描述列表
    let helps = HashMap::from([
        // MARK: main
        (
            CMD_INTO_FILEMONITOR,
            (CMD_INTO_FILEMONITOR, "进入文件监控器"),
        ),
        (CMD_HELP, (CMD_HELP, "查看帮助")),
        (CMD_QUIT, (CMD_QUIT, "退出")),
        // MARK: filemonitor
        (CMD_SHOW_STATUS, (CMD_SHOW_STATUS, "查看状态")),
        (CMD_SHOW_LOGS, (CMD_SHOW_LOGS, "查看日志")),
        (CMD_START_MONITOR, (CMD_START_MONITOR, "开始监控")),
        (CMD_STOP_MONITOR, (CMD_STOP_MONITOR, "停止监控")),
        (CMD_START_SCAN, (CMD_START_SCAN, "开始扫描")),
    ]);
    println!("命令列表：");

    let mut output_cmds: Vec<(&str, &str)> = Vec::new();
    cmds.iter().for_each(|c| {
        let (cmd, desc) = helps.get(c).unwrap();
        output_cmds.push((cmd, desc));
    });

    output_cmds.sort_by(|a, b| a.0.cmp(b.0));
    for (cmd, desc) in output_cmds {
        println!("  {:<10}  {}", cmd, desc);
    }
}
