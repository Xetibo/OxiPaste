use std::path::PathBuf;

use iced::{Element};
use oxiced::{theme::theme::OXITHEME, widgets::oxi_button::{self, ButtonVariant}};

use crate::{Message, SVG_PATH};

pub fn mk_svg(name: &'static str) -> PathBuf {
    SVG_PATH.join(name)
}

#[cfg(debug_assertions)]
pub fn svg_path() -> PathBuf {
    PathBuf::from("./assets")
}

#[cfg(not(debug_assertions))]
pub fn svg_path() -> PathBuf {
    use std::env;
    use std::path::Path;
    match env::current_exe() {
        Ok(exe_path) => exe_path
            .parent()
            .unwrap_or(&Path::new("/"))
            .join("../share/pixmaps/oxipaste"),
        Err(_) => PathBuf::from("./assets"),
    }
}

#[derive(Debug, Clone)]
pub enum FocusDirection {
    Up,
    Down,
}

impl FocusDirection {
    pub fn add(self, rhs: usize, length: usize) -> usize {
        match self {
            FocusDirection::Up => {
                if rhs > 0 {
                    rhs - 1
                } else {
                    length - 1
                }
            }
            FocusDirection::Down => {
                if length > 0 {
                    (rhs + 1) % length
                } else {
                    0
                }
            }
        }
    }
}

pub fn mk_content_button(
    focused_index: usize,
    current_index: usize,
    content: Element<Message>,
) -> iced::widget::Button<'_, Message> {
    oxi_button::button(content, ButtonVariant::Neutral)
        .on_press(Message::Copy(current_index as i32))
        .style(move |theme, status| {
            let is_focused = current_index == focused_index;
            let palette = OXITHEME;
            let default_style = oxi_button::neutral_button(theme, status);
            let background = if is_focused {
                Some(iced::Background::Color(palette.primary_bg_hover))
            } else {
                default_style.background
            };
            iced::widget::button::Style {
                background,
                ..default_style
            }
        })
        .padding(5.0)
}
