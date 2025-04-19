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
    fn render_list(
        items: &Vec<Rc<RefCell<MenuItem>>>,
        area: Rect,
        buf: &mut Buffer,
        index: Option<usize>,
    ) {
        if items.is_empty() {
            return;
        }
        let mut state = ListState::default();
        state.select(index);
        StatefulWidget::render(
            List::new(items.iter().map(|item| item.borrow().name.clone()))
                .highlight_style(SELECTED_STYLE),
            area,
            buf,
            &mut state,
        );
    }

    fn render_to_left(
        &self,
        children: &Vec<Rc<RefCell<MenuItem>>>,
        area: Rect,
        buf: &mut Buffer,
        index: Option<usize>,
    ) {
        Self::render_list(children, area, buf, index);
    }

    fn render_to_right(
        &self,
        children: &Vec<Rc<RefCell<MenuItem>>>,
        area: Rect,
        buf: &mut Buffer,
        index: Option<usize>,
    ) {
        Self::render_list(children, area, buf, index);
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
        match state.selected_indices.len() {
            // 未选中菜单
            0 => self.render_to_left(&self.children, left_area, buf, None),

            // 一级菜单
            1 => {
                // 若超出边界，则将选中的菜单项设置为最后一个
                let selected_index =
                    state.selected_indices[0].min(self.children.len().saturating_sub(1));
                self.render_to_left(&self.children, left_area, buf, Some(selected_index));

                if self.children[selected_index].borrow().children.len() > 0 {
                    self.render_to_right(
                        &self.children[selected_index].borrow().children,
                        right_area,
                        buf,
                        None,
                    );
                }
            }

            // 大于二级菜单
            _ => {
                let mut last_item = Rc::clone(&self.children[0].borrow().parent.upgrade().unwrap());

                // 获取最终选中的菜单项，清除异常项
                for i in 0..state.selected_indices.len() {
                    if last_item.borrow().children.len() == 0 {
                        state.selected_indices.truncate(i);
                        return;
                    } else {
                        state.selected_indices[i] = state.selected_indices[i]
                            .min(last_item.borrow().children.len().saturating_sub(1));
                        let tem_last_item =
                            Rc::clone(&last_item.borrow().children[state.selected_indices[i]]);

                        last_item = tem_last_item;
                    }
                }

                // 判断最终选中项是否有子菜单
                let parent_menu = last_item.borrow().parent.upgrade().unwrap();
                let grand_parent_menu = parent_menu.borrow().parent.upgrade().unwrap();

                let (left_children, right_children, left_idx, right_idx) =
                    if last_item.borrow().children.is_empty() {
                        let right_idx = state.selected_indices.last().unwrap();
                        let left_idx = state
                            .selected_indices
                            .last_chunk::<2>()
                            .map(|a| a[0])
                            .unwrap_or(0);
                        (
                            &grand_parent_menu.borrow().children,
                            &parent_menu.borrow().children,
                            left_idx,
                            Some(*right_idx),
                        )
                    } else {
                        let left_idx = state.selected_indices.last().unwrap();
                        (
                            &parent_menu.borrow().children,
                            &last_item.borrow().children,
                            *left_idx,
                            None,
                        )
                    };

                self.render_to_left(left_children, left_area, buf, Some(left_idx));
                self.render_to_right(right_children, right_area, buf, right_idx);
            }
        }
    }
}
