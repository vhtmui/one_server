use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct MenuState {
    pub selected: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Menu {
    pub name: String,
    pub children: Vec<MenuItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MenuItem {
    pub name: String,
    pub children: Vec<MenuItem>,
}

impl Menu {
    pub fn new(name: String, children: Vec<MenuItem>) -> Self {
        Menu { name, children }
    }

    // 从 JSON 字符串构建 Menu
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
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
          "children": []
        },
        {
          "name": "Settings",
          "children": [
            {
              "name": "Audio",
              "children": []
            },
            {
              "name": "Video",
              "children": []
            }
          ]
        }
      ]
    }
    "#;

    match Menu::from_json(json_data) {
        Ok(menu) => println!("Menu created: {:?}", menu),
        Err(e) => eprintln!("Failed to parse JSON: {}", e),
    }
}