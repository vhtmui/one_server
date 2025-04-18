use std::{cell::RefCell, rc::Rc};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{
        Block, Borders, List, ListState, StatefulWidget, StatefulWidgetRef, Widget, WidgetRef,
    },
};

use crate::{
    apps::{SELECTED_STYLE, file_monitor::dichotomize_area_with_midlines},
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
            StatefulWidget::render(
                List::new(children.iter().map(|child| child.borrow().name.clone()))
                    .highlight_style(SELECTED_STYLE),
                area,
                buf,
                &mut state,
            );
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
        let (left_area, midline, right_area) = dichotomize_area_with_midlines(
            area,
            Direction::Horizontal,
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        );

        Block::default().borders(Borders::LEFT).render(midline, buf);

        // 判断是否有选中的菜单项
        if state.selected_indices.len() == 0 {
            self.render_left(&self.children, left_area, buf, None);
        }
        // 判断选中的菜单项是否为第一级菜单
        else if state.selected_indices.len() == 1 {
            // 若超出边界，则将选中的菜单项设置为最后一个
            if state.selected_indices[0] > self.children.len().saturating_sub(1) {
                state.selected_indices[0] = self.children.len().saturating_sub(1);
            }

            let selected_index = state.selected_indices[0];

            self.render_left(&self.children, left_area, buf, Some(selected_index));

            // 如果选中项有子菜单，则渲染子菜单
            if self.children[selected_index].borrow().children.len() > 0 {
                self.render_right(
                    &self.children[selected_index].borrow().children,
                    right_area,
                    buf,
                    None,
                );
            }
        }
        // 判断选中的菜单项是否为大于第二级的菜单
        else if state.selected_indices.len() > 1 {
            // 选中最后一个菜单项的索引，并处理越界情况
            let mut last_item = Rc::clone(&self.children[state.selected_indices[0]]);

            let mut outbound_index: Option<usize> = None;
            for i in 1..state.selected_indices.len() {
                if state.selected_indices[i] > last_item.borrow().children.len() {
                    state.selected_indices[i] = last_item.borrow().children.len().saturating_sub(1);
                }

                let mut tem = Rc::new(RefCell::new(MenuItem::default()));

                if let Some(child) = last_item.borrow().children.get(state.selected_indices[i]) {
                    tem = Rc::clone(&child);
                }
                else {
                    outbound_index = Some(i);
                    break;
                };
                last_item = tem;
            }
            if let Some(index) = outbound_index {
                state.selected_indices.truncate(index);
            }

            // 判断最终选中项是否有子菜单
            if last_item.borrow().children.len() > 0 {
                let left_index = state.selected_indices.last().unwrap();
                let parent_menu = last_item.borrow().parent.upgrade().unwrap();
                self.render_left(
                    &parent_menu.borrow().children,
                    left_area,
                    buf,
                    Some(*left_index),
                );
                self.render_right(&last_item.borrow().children, right_area, buf, None);
            } else {
                let left_index = state.selected_indices.last_chunk::<2>().unwrap()[0];
                let parent_menu = last_item.borrow().parent.upgrade().unwrap();
                let grand_parent_menu = parent_menu.borrow().parent.upgrade().unwrap();
                self.render_left(
                    &grand_parent_menu.borrow().children,
                    left_area,
                    buf,
                    Some(left_index),
                );

                let right_index = state.selected_indices.last().unwrap();
                self.render_right(
                    &parent_menu.borrow().children,
                    right_area,
                    buf,
                    Some(*right_index),
                );
            }
        }
    }
}
