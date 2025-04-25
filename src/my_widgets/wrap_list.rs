use std::collections::VecDeque;

use hyphenation::{Language, Load, Standard};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, StatefulWidgetRef},
};
use textwrap::WordSplitter;

use crate::apps::{
    MENU_HIGHLIGHT_STYLE,
    file_monitor::{MonitorEvent, MonitorEventType},
};

pub struct WrapList<'a> {
    pub raw_list: VecDeque<MonitorEvent>,
    pub list: VecDeque<ListItem<'a>>,
    pub wrap_len: Option<usize>,
}

impl WrapList<'_> {
    pub fn new(capacity: usize) -> Self {
        Self {
            raw_list: VecDeque::with_capacity(capacity),
            list: VecDeque::with_capacity(capacity),
            wrap_len: None,
        }
    }

    pub fn add_raw_item(&mut self, item: MonitorEvent) {
        if self.list.len() == self.wrap_len.unwrap_or_default() {
            self.raw_list.pop_front();
        }
        self.raw_list.push_back(item);
    }

    pub fn add_item(&mut self, e: MonitorEvent) {
        let (prefix, color) = match e.event_type {
            MonitorEventType::Error => ("[ERR]  ", Color::Red),
            MonitorEventType::CreatedFile => ("[CREATE]", Color::Green),
            MonitorEventType::ModifiedFile => ("[MODIFY]", Color::Blue),
            MonitorEventType::DeletedFile => ("[DELETE]", Color::Magenta),
            MonitorEventType::StopMonitor => ("[STOP]", Color::Yellow),
            MonitorEventType::Info => ("[INFO]  ", Color::White),
        };

        let time_str = e
            .time
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "--:--:--".into());

        let text = format!("{prefix} {time_str} {}", e.message);

        let width = self.wrap_len.unwrap_or_default();

        let dictionary = Standard::from_embedded(Language::EnglishUS).unwrap();
        let options =
            textwrap::Options::new(width).word_splitter(WordSplitter::Hyphenation(dictionary));

        let wrapped_lines: Vec<String> = textwrap::wrap(&text, options)
            .iter()
            .map(|s| s.to_string())
            .collect();

        let lines: Vec<Line> = wrapped_lines
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                if index == 0 {
                    let parts: Vec<&str> = line.splitn(2, prefix).collect();
                    Line::from(vec![
                        Span::styled(prefix.to_string(), Style::new().fg(color)),
                        Span::from(parts[1].to_string()),
                    ])
                } else {
                    Line::from(line)
                }
            })
            .collect();

        if self.raw_list.len() == self.wrap_len.unwrap_or_default() {
            self.raw_list.pop_front();
        }

        self.list.push_back(ListItem::new(Text::from(lines)));
    }

    pub fn update_list(&mut self) {
        let width = self.wrap_len.unwrap_or_default();

        let items: Vec<ListItem> = self
            .raw_list
            .iter()
            .rev()
            .map(|e| {
                let (prefix, color) = match e.event_type {
                    MonitorEventType::Error => ("[ERR]  ", Color::Red),
                    MonitorEventType::CreatedFile => ("[CREATE]", Color::Green),
                    MonitorEventType::ModifiedFile => ("[MODIFY]", Color::Blue),
                    MonitorEventType::DeletedFile => ("[DELETE]", Color::Magenta),
                    MonitorEventType::StopMonitor => ("[STOP]", Color::Yellow),
                    MonitorEventType::Info => ("[INFO]  ", Color::White),
                };

                let time_str = e
                    .time
                    .map(|t| t.format("%H:%M:%S").to_string())
                    .unwrap_or_else(|| "--:--:--".into());

                let text = format!("{prefix} {time_str} {}", e.message);

                let dictionary = Standard::from_embedded(Language::EnglishUS).unwrap();
                let options = textwrap::Options::new(width)
                    .word_splitter(WordSplitter::Hyphenation(dictionary));

                let wrapped_lines: Vec<String> = textwrap::wrap(&text, options)
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                let lines: Vec<Line> = wrapped_lines
                    .into_iter()
                    .enumerate()
                    .map(|(index, line)| {
                        if index == 0 {
                            let parts: Vec<&str> = line.splitn(2, prefix).collect();
                            Line::from(vec![
                                Span::styled(prefix.to_string(), Style::new().fg(color)),
                                Span::from(parts[1].to_string()),
                            ])
                        } else {
                            Line::from(line)
                        }
                    })
                    .collect();

                ListItem::new(Text::from(lines))
            })
            .collect();

        self.list = items.into();
    }
}

impl StatefulWidget for &mut WrapList<'_> {
    type State = ListState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        if self.wrap_len == None {
            self.wrap_len = Some(area.width as usize);
        }
        let items = self.list.clone();
        StatefulWidgetRef::render_ref(
            &List::new(items)
                .block(Block::default().borders(Borders::NONE))
                .highlight_style(MENU_HIGHLIGHT_STYLE),
            area,
            buf,
            state,
        );
    }
}
