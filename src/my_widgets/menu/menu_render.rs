use std::{cell::RefCell, rc::Rc};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{List, ListState, StatefulWidget, StatefulWidgetRef, WidgetRef},
};

use crate::{
    apps::SELECTED_STYLE,
    my_widgets::menu::{MenuItem, MenuState},
};

impl MenuItem {
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
        self.render_left(children, area, buf, index);
    }
}

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
