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

pub struct WrapList {
    pub raw_list: VecDeque<MonitorEvent>,
    pub list: VecDeque<ListItem<'static>>,
    pub wrap_len: Option<usize>,
    // 新增：缓存分词字典避免重复加载
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

    // 新增：提取重复的列表项创建逻辑
    fn create_list_item(&self, e: &MonitorEvent) -> ListItem<'static> {
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
                    // 新增：处理splitn可能的空值情况
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

    // 修改：使用新创建方法简化代码
    pub fn add_item(&mut self, e: MonitorEvent) {
        let item = self.create_list_item(&e);
        self.list.push_back(item);
    }

    // 修改：使用新创建方法并优化转换逻辑
    pub fn update_list(&mut self) {
        let items: Vec<ListItem> = self
            .raw_list
            .iter()
            .rev()
            .map(|e| self.create_list_item(e))
            .collect();
        // 使用into_iter转换为VecDeque
        self.list = items.into_iter().collect();
    }

    // 修改：修复容量判断逻辑
    pub fn add_raw_item(&mut self, item: MonitorEvent) {
        let max_len = self.wrap_len.unwrap_or(usize::MAX);
        if self.list.len() == max_len {
            self.raw_list.pop_front();
        }
        self.raw_list.push_back(item.clone());

        self.add_item(item);
    }
}
// 修改：构造函数初始化字典并处理加载错误

// 修改：优化渲染时的宽度判断
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
