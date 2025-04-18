#[derive(Debug, Default, Clone)]
pub struct MenuState {
    pub selected_indices: Vec<usize>,
}

impl MenuState {
    pub fn select_up(&mut self) {
        if self.selected_indices.len() == 0 {
            self.select_right();
            return;
        }
        if let Some(index) = self.selected_indices.last_mut() {
            if *index > 0 {
                *index -= 1;
            }
        }
    }

    pub fn select_down(&mut self) {
        if self.selected_indices.len() == 0 {
            self.select_right();
            return;
        }
        if let Some(index) = self.selected_indices.last_mut() {
            *index += 1;
        }
    }

    pub fn select_left(&mut self) {
        if self.selected_indices.len() > 0 {
            self.selected_indices.pop();
        }
    }

    pub fn select_right(&mut self) {
        self.selected_indices.push(0);
    }
}
