use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, FixedOffset, Utc};
use notify::{Event as NotifyEvent, RecursiveMode, Result as NotifyResult, Watcher};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, read},
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Widget, WidgetRef},
};

use crate::{
    app::AppAction::{self, *},
    file_monitor::MonitorStatus::*,
    my_widgets::MyWidgets,
};

#[derive(Default)]
pub struct FileAnalyzer {
    files_watched: Vec<WatchFileInfo>,
    files_got: usize,
    files_recorded: usize,
}

pub struct WatchFileInfo {
    path: PathBuf,
    last_size: usize,
    last_byte_read_to: usize,
}

pub enum MonitorEventType {
    CreatedFile,
    ModifiedFile,
    DeletedFile,
}

#[derive(Clone, PartialEq, Eq)]
pub enum MonitorStatus {
    Running,
    Stopped,
    Paused,
    Error,
}

pub struct MonitorEvent {
    time: Option<DateTime<FixedOffset>>,
    event_type: MonitorEventType,
    message: String,
}

struct SharedState {
    lunch_time: Option<DateTime<FixedOffset>>,
    elapsed_time: Duration,
    status: MonitorStatus,
    file_analyzer: FileAnalyzer,
    events: VecDeque<MonitorEvent>,
}

impl SharedState {
    fn add_event(&mut self, event: MonitorEvent) {
        if self.events.len() == 10 {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
}

pub struct Monitor {
    path: String,
    shared_state: Arc<Mutex<SharedState>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Monitor {
    pub fn new(path: String) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            lunch_time: None,
            elapsed_time: Duration::from_secs(0),
            status: Stopped,
            file_analyzer: FileAnalyzer::default(),
            events: VecDeque::with_capacity(10),
        }));

        Monitor {
            path,
            shared_state,
            handle: None,
        }
    }

    pub fn start_monitor(&mut self) -> NotifyResult<()> {
        let mut locked_state = self.shared_state.lock().unwrap();
        locked_state.lunch_time =
            Some(Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap()));
        locked_state.status = Running;

        let path = self.path.clone();
        let cloned_shared_state = Arc::clone(&self.shared_state);
        let handle = thread::spawn(move || {
            if let Err(e) = Monitor::inner_monitor(cloned_shared_state, &path) {
                eprintln!("Error in file monitoring thread: {:?}", e);
            }
        });

        self.handle = Some(handle);

        Ok(())
    }

    fn inner_monitor(shared_state: Arc<Mutex<SharedState>>, path: &str) -> NotifyResult<()> {
        let (tx, rx) = mpsc::channel::<NotifyResult<NotifyEvent>>();

        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;

        loop {
            match rx.recv() {
                Ok(event) => {
                    print!("Event: {:?}\n", event);
                }
                Err(e) => {
                    eprintln!("Watch error: {:?}", e);
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn add_event(&mut self, event: MonitorEvent) {
        let mut locked_state = self.shared_state.lock().unwrap();
        if locked_state.events.len() == 10 {
            locked_state.events.pop_front();
        }
        locked_state.events.push_back(event);
    }

    fn analyze_content(content: &str) -> String {
        content.to_string()
    }

    pub fn get_status_text(&self) -> MonitorStatus {
        self.shared_state.lock().unwrap().status.clone()
    }
}

pub struct FileMonitor {
    title: String,
    monitor: Monitor,
}

impl FileMonitor {
    pub fn new(title: String, path: String) -> Self {
        FileMonitor {
            title: title,
            monitor: Monitor::new(path),
        }
    }

    pub fn get_layout_areas(&self, area: Rect) -> (Rect, Rect, Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[0]);

        (left_chunks[0], left_chunks[1], chunks[1])
    }

    pub fn render_control_panel(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::new().borders(Borders::ALL).title("control panel");
    }

    pub fn render_status_area(&self, area: Rect, buf: &mut Buffer) {}

    pub fn render_log_area(&self, area: Rect, buf: &mut Buffer) {}

    pub fn start_monitor(&mut self) {
        self.monitor.start_monitor().unwrap();
    }
}

impl WidgetRef for FileMonitor {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::new().borders(Borders::ALL).title(&*self.title);
        block.render(area, buf);
    }
}

impl MyWidgets for FileMonitor {
    fn handle_event(&mut self, event: Event) -> Result<AppAction, std::io::Error> {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Release,
            ..
        }) = event
        {
            match code {
                KeyCode::Esc => {
                    return Ok(ToggleMenu);
                }
                KeyCode::Enter => {
                    if let Event::Key(KeyEvent {
                        code: KeyCode::Enter,
                        kind: KeyEventKind::Press,
                        ..
                    }) = read().unwrap()
                    {
                        if self.monitor.get_status_text() == MonitorStatus::Stopped {
                            self.start_monitor();
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Default)
    }
}
