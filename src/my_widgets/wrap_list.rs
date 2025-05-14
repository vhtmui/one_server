use std::collections::VecDeque;

use hyphenation::{Language, Load, Standard};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, StatefulWidgetRef},
};
use textwrap::WordSplitter;

use crate::{
    DirScannerEventKind as DSE, EventKind::*, LogObserverEventKind as LOE, OneEvent,
    apps::MENU_HIGHLIGHT_STYLE,
};

#[derive(Clone)]
pub struct WrapList {
    raw_list: VecDeque<OneEvent>,
    list: VecDeque<ListItem<'static>>,
    wrap_len: Option<usize>,
    dictionary: Standard,
}

impl WrapList {
    pub fn new(capacity: usize) -> Self {
        let dictionary = Standard::from_embedded(Language::EnglishUS)
            .expect("Failed to load EnglishUS hyphenation dictionary");
        Self {
            raw_list: VecDeque::with_capacity(capacity),
            list: VecDeque::with_capacity(capacity),
            wrap_len: None,
            dictionary,
        }
    }

    pub fn with_raw_list(mut self, raw_list: VecDeque<OneEvent>) -> Self {
        self.raw_list = raw_list;
        self.update_list();
        self
    }

    pub fn create_text(e: &OneEvent) -> (&str, String, Color) {
        let (prefix, color) = match &e.kind {
            LogObserverEvent(l) => match l {
                LOE::Error => ("[OBSERVER][ERR]  ", Color::Red),
                LOE::CreatedFile => ("[OBSERVER][CREATE]", Color::Green),
                LOE::ModifiedFile => ("[OBSERVER][MODIFY]", Color::Blue),
                LOE::DeletedFile => ("[OBSERVER][DELETE]", Color::Magenta),
                LOE::Info => ("[OBSERVER][INFO]  ", Color::Magenta),
                LOE::Start => ("[OBSERVER][START]  ", Color::Cyan),
                LOE::Stop => ("[OBSERVER][STOP]  ", Color::Red),
            },

            DirScannerEvent(d) => match d {
                DSE::Start => ("[SCANNER][SCAN]  ", Color::Cyan),
                DSE::Stop => ("[SCANNER][STOP]  ", Color::Yellow),
                DSE::Complete => ("[SCANNER][COMPLETE]", Color::Green),
                DSE::Error => ("[SCANNER][ERR]  ", Color::Red),
                DSE::Info => ("[SCANNER][INFO]  ", Color::Magenta),
                DSE::DBInfo => ("[SCANNER][DBINFO]", Color::Blue),
            },
        };

        let time_str = e
            .time
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "--:--:--".into());

        let text = format!("{prefix} {time_str} {}", e.content);
        (prefix, text, color)
    }

    /// Create a ListItem from a MonitorEvent, use `self.wrap_len`` and `self.dictionary` to wrap the text.
    fn create_list_item(&self, e: &OneEvent) -> ListItem<'static> {
        let (prefix, text, color) = Self::create_text(e);

        let options = textwrap::Options::new(self.wrap_len.unwrap_or(usize::MAX))
            .word_splitter(WordSplitter::Hyphenation(self.dictionary.clone()));

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
                    if parts.len() < 2 {
                        panic!("Unexpected line format when splitting prefix: {}", line);
                    }
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
    }

    /// Add ListItem to `self.list`.
    pub fn add_item(&mut self, e: OneEvent) {
        let item = self.create_list_item(&e);
        self.list.push_front(item);
        if self.list.len() > self.wrap_len.unwrap_or(500) {
            self.list.pop_back();
        }
    }

    /// Update `self.list` from `self.raw_list`.
    pub fn update_list(&mut self) {
        let items: Vec<ListItem> = self
            .raw_list
            .iter()
            .map(|e| self.create_list_item(e))
            .collect();
        self.list = items.into_iter().collect();
    }

    /// Add raw item of MonitorEvent to `self.raw_list`.
    pub fn add_raw_item(&mut self, item: OneEvent) {
        let max_len = self.wrap_len.unwrap_or(500);
        if self.list.len() == max_len {
            self.raw_list.pop_back();
        }
        self.raw_list.push_front(item.clone());

        self.add_item(item);
    }

    pub fn get_raw_list(&self) -> VecDeque<OneEvent> {
        self.raw_list.clone()
    }

    pub fn get_raw_list_string(&self) -> Vec<String> {
        self.raw_list
            .iter()
            .map(|e| {
                let (_, text, _) = Self::create_text(e);
                format!("{text}")
            })
            .collect()
    }
}

impl StatefulWidget for &mut WrapList {
    type State = ListState;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let current_width = area.width as usize;
        if self.wrap_len != Some(current_width) {
            self.wrap_len = Some(current_width);
            self.update_list();
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
