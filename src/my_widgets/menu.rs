pub use self::menu_state::MenuState;

mod menu_render;
pub mod menu_state;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Block,
};
use serde::{Deserialize, Serialize};

// 定义一个辅助结构体，用于序列化和反序列化 MenuItem
#[derive(Serialize, Deserialize, Debug)]
pub struct SerializableMenuItem {
    pub name: String,
    pub content: String,
    pub children: Vec<SerializableMenuItem>,
}

#[derive(Default, Debug)]
pub struct MenuItem<'a> {
    name: String,
    content: String,
    children: Vec<Rc<RefCell<MenuItem<'a>>>>,
    selected: bool,
    parent: Weak<RefCell<MenuItem<'a>>>,
    block: Option<Block<'a>>,
}

impl<'a> MenuItem<'a> {
    pub fn new(
        name: String,
        content: String,
        children: Vec<Rc<RefCell<MenuItem<'a>>>>,
        parent: Weak<RefCell<MenuItem<'a>>>,
    ) -> Self {
        MenuItem {
            name,
            content,
            children,
            selected: false,
            parent,
            block: None,
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
            block: None,
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
    pub fn set_block(&mut self, block: Block<'a>){
        self.block = Some(block);
    }
}

impl<'a> PartialEq for MenuItem<'a> {
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

impl<'a> Eq for MenuItem<'a> {}

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
