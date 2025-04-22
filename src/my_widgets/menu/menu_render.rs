use std::{cell::RefCell, rc::Rc};

use crossterm::style;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color::*, Modifier, Style, palette::material::YELLOW},
    widgets::{
        Block, Borders, List, ListState, StatefulWidget, StatefulWidgetRef, Widget, WidgetRef,
    },
};

use crate::my_widgets::{
    dichotomize_area_with_midlines,
    menu::{MenuItem, MenuState},
};

pub const MENU_HIGHLIGHT: Style = Style::new()
    .bg(Indexed(30))
    .add_modifier(Modifier::BOLD);
pub const MENU_SELECTED: Style = Style::new().fg(Red).bg(Indexed(43));

impl MenuItem {
    fn render_list(
        items: &Vec<Rc<RefCell<MenuItem>>>,
        area: Rect,
        buf: &mut Buffer,
        index: Option<usize>,
        style: Style,
    ) {
        if items.is_empty() {
            return;
        }
        let mut state = ListState::default();
        state.select(index);
        StatefulWidget::render(
            List::new(items.iter().map(|item| item.borrow().name.clone())).highlight_style(style),
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
        Self::render_list(children, area, buf, index, MENU_HIGHLIGHT);
    }

    fn render_to_right(
        &self,
        children: &Vec<Rc<RefCell<MenuItem>>>,
        area: Rect,
        buf: &mut Buffer,
        index: Option<usize>,
    ) {
        Self::render_list(children, area, buf, index, MENU_SELECTED);
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

        Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::new().add_modifier(Modifier::DIM))
            .render(midline, buf);

        // 判断是否有选中的菜单项
        match state.selected_indices.len() {
            // 未选中菜单
            0 => self.render_to_left(&self.children, left_area, buf, None),

            // 一级菜单
            1 => {
                // 若超出边界，则将选中的菜单项设置为最后一个
                let selected_index =
                    state.selected_indices[0].min(self.children.len().saturating_sub(1));
                state.selected_indices[0] = selected_index;
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
