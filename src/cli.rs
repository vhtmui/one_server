use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::PathBuf,
    vec,
};

use std::time::Duration;

use crate::{
    apps::file_sync_manager::SyncEngine,
    my_widgets::{LogKind, MyWidgets},
    *,
};

// 命令常量定义
pub const CMD_QUIT: &str = ":q";
pub const CMD_HELP: &str = "ls";
pub const CMD_INTO_FILESYNC_MGR: &str = "cd fm";
pub const CMD_START_OBS: &str = "start obs";
pub const CMD_STOP_OBS: &str = "stop obs";
pub const CMD_START_SCAN: &str = "start sc";
pub const CMD_START_PERIODIC_SCAN: &str = "start psc";
pub const CMD_SHOW_STATUS: &str = "ds status";
pub const CMD_SHOW_OBS_LOGS: &str = "ds log obs";
pub const CMD_SHOW_SCAN_LOGS: &str = "ds log sc";
pub const CMD_INPUT_DIR: &str = "<dir>";
pub const CMD_INPUT_INTERVAL: &str = "<interval>";

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
                help(vec![CMD_INTO_FILESYNC_MGR]);
            }
            CMD_INTO_FILESYNC_MGR => {
                into_file_sync_mgr();
            }
            "" => {}
            _ => println!("未知命令，输入 help 查看帮助"),
        }
    }
    println!("已退出命令行模式。");
}

fn into_file_sync_mgr() {
    // 创建文件监控器
    let path = load_config().file_sync_manager.observed_path;
    let mut file_sync_manager = SyncEngine::new("file_monitor".to_string(), path, 50);
    loop {
        let cmd = read_trimmed_line("\\filemonitor> ").unwrap_or_else(|| {
            println!("读取输入失败");
            "".to_string()
        });
        match cmd.as_str() {
            CMD_QUIT => break,
            CMD_HELP => {
                help(vec![
                    CMD_QUIT,
                    CMD_HELP,
                    CMD_SHOW_STATUS,
                    CMD_SHOW_OBS_LOGS,
                    CMD_START_SCAN,
                    CMD_START_PERIODIC_SCAN,
                    CMD_START_OBS,
                    CMD_STOP_OBS,
                ]);
            }
            CMD_SHOW_STATUS => {
                println!("监控器状态：{:?}", file_sync_manager.observer.get_status());
                println!("扫描器状态：{:?}", file_sync_manager.scanner.get_status());
            }
            CMD_SHOW_OBS_LOGS => {
                println!("日志：");
                for log in file_sync_manager.get_logs_str(LogKind::Observer) {
                    println!("{}", log);
                }
            }
            CMD_SHOW_SCAN_LOGS => {
                println!("扫描日志：");
                for log in file_sync_manager.get_logs_str(LogKind::Scanner) {
                    println!("{}", log);
                }
            }
            CMD_START_SCAN => {
                println!("  输入扫描路径：");
                loop {
                    let path = read_trimmed_line("").unwrap_or_else(|| {
                        println!("读取输入失败");
                        "".to_string()
                    });
                    match path.as_str() {
                        "" => {
                            println!("  输入为空，请重新输入");
                            continue;
                        }
                        CMD_QUIT => break,
                        CMD_HELP => {
                            help(vec![CMD_QUIT, CMD_HELP, CMD_INPUT_DIR]);
                            continue;
                        }
                        path => {
                            if fs::metadata(path).is_ok() {
                                file_sync_manager.scanner.set_path(PathBuf::from(path));
                                file_sync_manager.scanner.start_scanner().unwrap();
                                println!("开始扫描目录：{}", path);
                                break;
                            } else {
                                print!("目录不存在，请重新输入: ");
                            }
                        }
                    }
                }
            }
            CMD_START_PERIODIC_SCAN => {
                println!("输入路径");
                loop {
                    let path = read_trimmed_line("").unwrap_or_else(|| {
                        println!("读取输入失败");
                        "".to_string()
                    });

                    match path.as_str() {
                        "" => {
                            println!("输入为空，请重新输入");
                            continue;
                        }
                        CMD_QUIT => break,
                        CMD_HELP => {
                            help(vec![CMD_QUIT, CMD_HELP, CMD_INPUT_DIR]);
                            continue;
                        }
                        path => {
                            if fs::metadata(&path).is_ok() {
                                file_sync_manager
                                    .scanner
                                    .set_path(PathBuf::from(path));
                                println!("输入时间间隔（单位：分钟）");
                                loop {
                                    let interval = read_trimmed_line("").unwrap_or_else(|| {
                                        println!("读取输入失败");
                                        "".to_string()
                                    });
                                    match interval.as_str() {
                                        "" => {
                                            println!("时间间隔不能为空，请重新输入");
                                            continue;
                                        }
                                        CMD_QUIT => break,
                                        CMD_HELP => {
                                            help(vec![CMD_QUIT, CMD_HELP, CMD_INPUT_INTERVAL]);
                                            continue;
                                        }
                                        _ => {}
                                    }
                                    if interval.is_empty() {
                                        println!("时间间隔不能为空，请重新输入");
                                        continue;
                                    }
                                    if let Ok(interval) = interval.parse::<u64>() {
                                        file_sync_manager.scanner.start_periodic_scan(
                                            Duration::from_secs(interval * 60),
                                        );
                                        println!("开始定时扫描目录：{}", path);
                                        break;
                                    } else {
                                        println!("时间间隔格式错误，请重新输入");
                                    }
                                }
                                break;
                            } else {
                                print!("目录不存在，请重新输入: ");
                            }
                        }
                    }
                }
            }
            CMD_START_OBS => {
                println!(" 开始监控...");
                file_sync_manager.observer.start_observer().unwrap();
            }
            CMD_STOP_OBS => {
                println!(" 停止监控...");
                file_sync_manager.observer.stop_observer();
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
            CMD_INTO_FILESYNC_MGR,
            (CMD_INTO_FILESYNC_MGR, "进入文件监控器"),
        ),
        (CMD_HELP, (CMD_HELP, "查看帮助")),
        (CMD_QUIT, (CMD_QUIT, "退出")),
        // MARK: filemonitor
        (CMD_SHOW_STATUS, (CMD_SHOW_STATUS, "查看状态")),
        (CMD_SHOW_OBS_LOGS, (CMD_SHOW_OBS_LOGS, "查看日志")),
        (CMD_START_OBS, (CMD_START_OBS, "开始监控")),
        (CMD_STOP_OBS, (CMD_STOP_OBS, "停止监控")),
        (CMD_START_SCAN, (CMD_START_SCAN, "开始扫描")),
        (
            CMD_START_PERIODIC_SCAN,
            (CMD_START_PERIODIC_SCAN, "开始定时扫描"),
        ),
        (CMD_INPUT_DIR, (CMD_INPUT_DIR, "输入目录")),
        (CMD_INPUT_INTERVAL, (CMD_INPUT_INTERVAL, "输入时间间隔 (单位：分钟)")),
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
