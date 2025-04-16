pub struct MenuState {
    pub selected: Vec<usize>,
}

pub struct Menu {
    pub name: String,
    pub groups: Vec<MenuGroup>,
}

pub struct MenuGroup {
    pub name: String,
    pub items: Vec<MenuItem>,
}

pub struct MenuItem {
    pub name: String,
}

impl Menu {
    pub fn new(name: String, groups: Vec<MenuGroup>) -> Self {
        Menu {
            name,
            groups,
        }
    }
}

impl MenuGroup {
    pub fn new(name: String, items: Vec<MenuItem>) -> Self {
        MenuGroup {
            name,
            items,
        }
    }
}