use std::cell::RefCell;
use std::rc::{Rc, Weak};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{List, ListState, StatefulWidget, StatefulWidgetRef, WidgetRef},
};
use serde::{Deserialize, Serialize};

use crate::{apps::SELECTED_STYLE, my_widgets::MyWidgets};

// 定义一个辅助结构体，用于序列化和反序列化 MenuItem
#[derive(Serialize, Deserialize, Debug)]
struct SerializableMenuItem {
    pub name: String,
    pub content: String,
    pub children: Vec<SerializableMenuItem>,
}

#[derive(Default, Debug)]
pub struct MenuItem {
    name: String,
    content: String,
    children: Vec<Rc<RefCell<MenuItem>>>,
    selected: bool,
    parent: Weak<RefCell<MenuItem>>,
}

// 定义 MenuState 结构体
#[derive(Debug, Default, Clone)]
pub struct MenuState {
    pub selected_indices: Vec<usize>,
}

impl MenuState {
    
}

impl MenuItem {
    pub fn new(
        name: String,
        content: String,
        children: Vec<Rc<RefCell<MenuItem>>>,
        parent: Weak<RefCell<MenuItem>>,
    ) -> Self {
        MenuItem {
            name,
            content,
            children,
            selected: false,
            parent,
        }
    }

    // 从 JSON 字符串反序列化为 MenuItem
    pub fn from_json(json_str: &str) -> Result<Rc<RefCell<MenuItem>>, serde_json::Error> {
        let serializable_item: SerializableMenuItem = serde_json::from_str(json_str)?;
        Ok(Self::from_serializable(serializable_item, Weak::new()))
    }

    // 序列化 MenuItem 为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let serializable_item = self.to_serializable();
        serde_json::to_string(&serializable_item)
    }

    // 从可序列化的形式重建 MenuItem
    fn from_serializable(
        item: SerializableMenuItem,
        parent: Weak<RefCell<MenuItem>>,
    ) -> Rc<RefCell<MenuItem>> {
        let rc_item = Rc::new(RefCell::new(MenuItem {
            name: item.name,
            content: item.content,
            children: Vec::new(),
            selected: false,
            parent,
        }));

        let mut children = Vec::new();
        for child in item.children {
            children.push(Self::from_serializable(child, Rc::downgrade(&rc_item)));
        }

        rc_item.borrow_mut().children = children;
        rc_item
    }

    // 将 MenuItem 转换为可序列化的形式
    fn to_serializable(&self) -> SerializableMenuItem {
        SerializableMenuItem {
            name: self.name.clone(),
            content: self.content.clone(),
            children: self
                .children
                .iter()
                .map(|child| child.borrow().to_serializable())
                .collect(),
        }
    }

    fn render_left(
        &self,
        children: &Vec<Rc<RefCell<MenuItem>>>,
        area: Rect,
        buf: &mut Buffer,
        index: Option<usize>,
    ) {
        if children.is_empty() {
            return;
        } else {
            let mut state = ListState::default();
            state.select(index);
            List::new(children.iter().map(|child| child.borrow().name.clone()))
                .highlight_style(SELECTED_STYLE)
                .render(area, buf, &mut state);
        }
    }
    fn render_right(
        &self,
        children: &Vec<Rc<RefCell<MenuItem>>>,
        area: Rect,
        buf: &mut Buffer,
        index: Option<usize>,
    ) {
        if children.is_empty() {
            return;
        } else {
            let mut state = ListState::default();
            state.select(index);
            List::new(children.iter().map(|child| child.borrow().name.clone()))
                .highlight_style(SELECTED_STYLE)
                .render(area, buf, &mut state);
        }
    }
}

impl PartialEq for MenuItem {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.content == other.content
            && self.selected == other.selected
            && self.children.len() == other.children.len()
            && self
                .children
                .iter()
                .zip(other.children.iter())
                .all(|(a, b)| *a.borrow() == *b.borrow())
    }
}

impl Eq for MenuItem {}

impl WidgetRef for MenuItem {
    fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {}
}

impl StatefulWidgetRef for MenuItem {
    type State = MenuState;
    fn render_ref(
        &self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // 判断是否有选中的菜单项
        if state.selected_indices.len() > 0 {
            // 判断选中的菜单项是否为第一级菜单
            if state.selected_indices.len() == 1 {
                let index = state.selected_indices[0];

                self.render_left(&self.children, chunks[0], buf, Some(index));

                if self.children[index].borrow().children.len() > 0 {
                    self.render_right(
                        &self.children[index].borrow().children,
                        chunks[1],
                        buf,
                        None,
                    );
                }
            } else if state.selected_indices.len() > 1 {
                let mut last_item = &Rc::new(RefCell::new(MenuItem::default()));
                for i in 0..state.selected_indices.len() - 1 {
                    last_item = &self.children[state.selected_indices[i]];
                }

                // 判断选中项是否有子菜单
                if last_item.borrow().children.len() > 0 {
                    let left_index = state.selected_indices.last().unwrap();
                    self.render_left(
                        &last_item
                            .borrow()
                            .parent
                            .upgrade()
                            .unwrap()
                            .borrow()
                            .children,
                        chunks[0],
                        buf,
                        Some(*left_index),
                    );
                    self.render_right(&last_item.borrow().children, chunks[1], buf, None);
                } else {
                    let left_index = state.selected_indices.last_chunk::<2>().unwrap()[0];
                    self.render_left(
                        &last_item
                            .borrow()
                            .parent
                            .upgrade()
                            .unwrap()
                            .borrow()
                            .parent
                            .upgrade()
                            .unwrap()
                            .borrow()
                            .children,
                        area,
                        buf,
                        Some(left_index),
                    );
                    let right_index = state.selected_indices.last().unwrap();
                    self.render_right(
                        &last_item
                            .borrow()
                            .parent
                            .upgrade()
                            .unwrap()
                            .borrow()
                            .children,
                        area,
                        buf,
                        Some(*right_index),
                    );
                }
            }
        } else {
            self.render_left(&self.children, area, buf, Some(1));
        }
    }
}

#[test]
fn test_menu_builder() {
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

    let root = MenuItem::from_json(json_data).unwrap();

    // 验证根节点
    assert_eq!(root.borrow().name, "Main Menu");
    assert_eq!(root.borrow().content, "This is the main menu.");
    assert_eq!(root.borrow().children.len(), 2);

    // 验证 Home 节点
    let home = &root.borrow().children[0];
    assert_eq!(home.borrow().name, "Home");
    assert_eq!(home.borrow().content, "This is the home page.");
    assert_eq!(home.borrow().children.len(), 0);
    assert!(home.borrow().parent.upgrade().is_some());

    // 验证 Settings 节点
    let settings = &root.borrow().children[1];
    assert_eq!(settings.borrow().name, "Settings");
    assert_eq!(settings.borrow().content, "This is the settings page.");
    assert_eq!(settings.borrow().children.len(), 2);
    assert!(settings.borrow().parent.upgrade().is_some());

    // 验证 Audio 节点
    let audio = &settings.borrow().children[0];
    assert_eq!(audio.borrow().name, "Audio");
    assert_eq!(audio.borrow().content, "This is the audio settings page.");
    assert_eq!(audio.borrow().children.len(), 0);
    assert!(audio.borrow().parent.upgrade().unwrap().borrow().name == "Settings");

    // 验证 Video 节点
    let video = &settings.borrow().children[1];
    assert_eq!(video.borrow().name, "Video");
    assert_eq!(video.borrow().content, "This is the video settings page.");
    assert_eq!(video.borrow().children.len(), 0);
    assert!(video.borrow().parent.upgrade().unwrap().borrow().name == "Settings");
}
