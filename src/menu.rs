use ratatui::layout::{Constraint, Direction, Layout};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use ratatui::widgets::WidgetRef;

use crate::my_widgets::MyWidgets;

// 定义一个辅助结构体，用于序列化和反序列化 Menu
#[derive(Serialize, Deserialize, Debug)]
struct SerializableMenu {
    pub name: String,
    pub children: Vec<SerializableMenuItem>,
}

// 定义一个辅助结构体，用于序列化和反序列化
#[derive(Serialize, Deserialize, Debug)]
struct SerializableMenuItem {
    pub name: String,
    pub content: String,
    pub children: Vec<SerializableMenuItem>,
}

#[derive(Debug)]
pub struct Menu {
    pub name: String,
    pub children: Vec<Rc<RefCell<MenuItem>>>,
}

#[derive(Debug)]
pub struct MenuItem {
    pub name: String,
    pub content: String,
    pub children: Vec<Rc<RefCell<MenuItem>>>,
    pub selected: bool,
    pub parent: Weak<RefCell<MenuItem>>,
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

impl Menu {
    pub fn new(name: String, children: Vec<Rc<RefCell<MenuItem>>>) -> Self {
        Menu { name, children }
    }

    // 从 JSON 字符串反序列化为 Menu
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let serializable_menu: SerializableMenu = serde_json::from_str(json_str)?;
        Ok(Self::build_menu(serializable_menu))
    }

    fn build_menu(menu: SerializableMenu) -> Self {
        let mut new_children = Vec::new();
        for item in menu.children {
            new_children.push(MenuItem::from_serializable(item, Weak::new()));
        }
        Menu {
            name: menu.name,
            children: new_children,
        }
    }

    // 序列化 Menu 为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let serializable_children: Vec<SerializableMenuItem> = self
            .children
            .iter()
            .map(|child| child.borrow().to_serializable())
            .collect();
        let serializable_menu = SerializableMenu {
            name: self.name.clone(),
            children: serializable_children,
        };
        serde_json::to_string(&serializable_menu)
    }
}

impl WidgetRef for Menu {
    fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        
    }
}

#[test]
fn test_menu_builder() {
    let json_data = r#"
        {
          "name": "Main Menu",
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

    let menu = Menu::from_json(json_data).unwrap();

    // 验证根节点
    assert_eq!(menu.name, "Main Menu");
    assert_eq!(menu.children.len(), 2);

    // 验证 Home 节点
    let home = &menu.children[0];
    assert_eq!(home.borrow().name, "Home");
    assert_eq!(home.borrow().content, "This is the home page.");
    assert_eq!(home.borrow().children.len(), 0);
    assert!(home.borrow().parent.upgrade().is_none());

    // 验证 Settings 节点
    let settings = &menu.children[1];
    assert_eq!(settings.borrow().name, "Settings");
    assert_eq!(settings.borrow().content, "This is the settings page.");
    assert_eq!(settings.borrow().children.len(), 2);
    assert!(settings.borrow().parent.upgrade().is_none());

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
