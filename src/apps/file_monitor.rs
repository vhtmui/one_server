use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self};
use std::time::Duration;

use chrono::{DateTime, FixedOffset, Utc};
use notify::{Event as NotifyEvent, RecursiveMode, Result as NotifyResult, Watcher};
use ratatui::widgets::StatefulWidgetRef;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, read},
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Widget, WidgetRef},
};

use crate::{
    apps::AppAction::{self, *},
    apps::file_monitor::MonitorStatus::*,
    my_widgets::{
        MyWidgets,
        menu::{MenuItem, MenuState},
    },
};

pub struct FileMonitor {
    title: String,
    menu_state: RefCell<MenuState>,
    monitor: Monitor,
}

pub struct Monitor {
    path: String,
    shared_state: Arc<Mutex<SharedState>>,
    handle: Option<thread::JoinHandle<()>>,
}

struct SharedState {
    lunch_time: Option<DateTime<FixedOffset>>,
    elapsed_time: Duration,
    status: MonitorStatus,
    file_analyzer: FileAnalyzer,
    events: VecDeque<MonitorEvent>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum MonitorStatus {
    Running,
    Stopped,
    Paused,
    Error,
}

#[derive(Default)]
pub struct FileAnalyzer {
    files_watched: Vec<FileWhatchInfo>,
    files_got: usize,
    files_recorded: usize,
}

pub struct FileWhatchInfo {
    path: PathBuf,
    last_size: usize,
    last_byte_read_to: usize,
}

pub struct MonitorEvent {
    time: Option<DateTime<FixedOffset>>,
    event_type: MonitorEventType,
    message: String,
}

pub enum MonitorEventType {
    CreatedFile,
    ModifiedFile,
    DeletedFile,
}

impl FileMonitor {
    pub fn new(title: String, path: String) -> Self {
        FileMonitor {
            menu_state: RefCell::new(MenuState::default()),
            title: title,
            monitor: Monitor::new(path),
        }
    }

    pub fn render_control_panel(&self, area: Rect, buf: &mut Buffer) {
        self.render_block("Control Panel".to_string(), area, buf);
        self.render_menu(area, buf);
    }

    pub fn render_status_area(&self, area: Rect, buf: &mut Buffer) {
        self.render_block("Status Area".to_string(), area, buf);
    }

    pub fn render_log_area(&self, area: Rect, buf: &mut Buffer) {
        self.render_block("Log Area".to_string(), area, buf);
    }

    pub fn start_monitor(&mut self) {
        self.monitor.start_monitor().unwrap();
    }

    pub fn render_block(&self, title: String, area: Rect, buf: &mut Buffer) {
        let block = Block::new().title(title);
        block.render(area, buf);
    }

    pub fn render_menu(&self, area: Rect, buf: &mut Buffer) {
        let json_data = r#"
        {
          "name": "Main Menu",
          "content": "This is the main menu.",
          "children": [
            {
              "name": "Home",
              "content": "This is the home page.",
              "children": []
            },
            {
              "name": "Settings",
              "content": "This is the settings page.",
              "children": [
                {
                  "name": "Audio",
                  "content": "This is the audio settings page.",
                  "children": []
                },
                {
                  "name": "Video",
                  "content": "This is the video settings page.",
                  "children": []
                }
              ]
            }
          ]
        }
        "#;

        let menu_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)].as_ref())
            .split(area)[1];

        let mut state = self.menu_state.borrow_mut();

        if let Ok(menu_item) = MenuItem::from_json(json_data) {
            StatefulWidgetRef::render_ref(&*menu_item.borrow(), menu_area, buf, &mut *state);
        }
    }
}

impl WidgetRef for FileMonitor {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let (left_area, midline, right_area) = dichotomize_area_with_midlines(
            area,
            Direction::Horizontal,
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        );

        let (left_up_area, left_midline, left_down_area) = dichotomize_area_with_midlines(
            left_area,
            Direction::Vertical,
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        );

        self.render_control_panel(left_up_area, buf);
        self.render_status_area(left_down_area, buf);
        self.render_log_area(right_area, buf);
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
                    // if self.monitor.get_status_text() == MonitorStatus::Stopped {
                    //     self.start_monitor();
                    // }
                }
                KeyCode::Up => {
                    self.menu_state.borrow_mut().select_up();
                }
                KeyCode::Down => {
                    self.menu_state.borrow_mut().select_down();
                }
                KeyCode::Left => {
                    self.menu_state.borrow_mut().select_left();
                }
                KeyCode::Right => {
                    self.menu_state.borrow_mut().select_right();
                }
                _ => {}
            }
        }

        Ok(Default)
    }
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

impl SharedState {
    fn add_event(&mut self, event: MonitorEvent) {
        if self.events.len() == 10 {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
}

pub fn dichotomize_area_with_midlines(
    area: Rect,
    direction: Direction,
    left_constraint: Constraint,
    right_constraint: Constraint,
) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(direction)
        .constraints([left_constraint, Constraint::Length(1), right_constraint].as_ref())
        .split(area);

    (chunks[0], chunks[1], chunks[2])
}
