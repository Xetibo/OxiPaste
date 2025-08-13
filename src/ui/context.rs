use std::{fmt::Display, process::Command};

use iced::{
    futures,
    widget::{Button, rich_text, span},
};
use oxiced::widgets::{
    oxi_button::{ButtonVariant, button},
    oxi_svg::{self, SvgStyleVariant},
};

use crate::{
    Message, OxiPaste, OxiPasteError,
    config::Config,
    copy_to_clipboard,
    custom_rich::CustomRich,
    into_general_error,
    utils::{mk_content_button, mk_svg},
};

#[derive(Debug, Clone)]
pub struct Address {
    pub(crate) inner: String,
}

impl Address {
    pub fn try_build(value: String) -> Result<Self, String> {
        if value.starts_with("https://") || value.starts_with("/") || value.starts_with("./") {
            Ok(Self { inner: value })
        } else {
            // abuse of error lul
            Err(value)
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[derive(Debug, Clone)]
pub enum ImageContext {
    Regular(Vec<u8>),
}

impl ImageContext {
    fn apply_value(commands: Vec<Vec<String>>) -> Vec<Result<ContextCommand, OxiPasteError>> {
        commands
            .into_iter()
            .map(|command| {
                let res = ContextCommand::from_vec_and_value(command, "", true)?;
                Ok(res)
            })
            .collect()
    }

    // TODO fix duplication
    pub fn get_view_buttons(
        &self,
        focused_index: usize,
        current_index: usize,
        key: i32,
    ) -> (Button<Message>, Option<Button<Message>>) {
        match self {
            Self::Regular(image_content) => {
                let handle = iced::widget::image::Handle::from_bytes(image_content.clone());
                (
                    mk_content_button(
                        focused_index,
                        current_index,
                        iced::widget::image(handle).into(),
                    ),
                    Some(
                        button(
                            oxi_svg::svg_from_path(
                                SvgStyleVariant::Primary,
                                mk_svg("threedot.svg"),
                            ),
                            ButtonVariant::Neutral,
                        )
                        .on_press(Message::SubMessageContext(key, ContextMenuMessage::Expand))
                        .height(45)
                        .width(45),
                    ),
                )
            }
        }
    }
    pub fn get_context_actions(
        &self,
        config: &Config,
    ) -> Vec<Result<ContextCommand, OxiPasteError>> {
        match self {
            // TODO perhaps copy to clipboard here instead?
            // right now it requires a call to dbus...
            Self::Regular(_) => ImageContext::apply_value(config.ImageContextActions.clone()),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TextContext {
    Address(Address),
    Text(String),
}

impl TextContext {
    fn apply_value(
        commands: Vec<Vec<String>>,
        value: &str,
    ) -> Vec<Result<ContextCommand, OxiPasteError>> {
        commands
            .into_iter()
            .map(|command| {
                let res = ContextCommand::from_vec_and_value(command, value, false)?;
                Ok(res)
            })
            .collect()
    }

    pub fn get_view_buttons(
        &self,
        focused_index: usize,
        current_index: usize,
        key: i32,
    ) -> (Button<Message>, Option<Button<Message>>) {
        let text = match self {
            TextContext::Address(address) => &address.inner,
            TextContext::Text(text) => text,
        };
        (
            mk_content_button(
                focused_index,
                current_index,
                CustomRich::<(), Message>::custom_rich(rich_text![
                    span(text.to_owned()).underline(false)
                ])
                .into(),
            ),
            Some(
                button(
                    oxi_svg::svg_from_path(SvgStyleVariant::Primary, mk_svg("threedot.svg")),
                    ButtonVariant::Neutral,
                )
                .on_press(Message::SubMessageContext(key, ContextMenuMessage::Expand))
                .height(45)
                .width(45),
            ),
        )
    }
    // improve this via using actions instead
    pub fn get_context_actions(
        &self,
        config: &Config,
    ) -> Vec<Result<ContextCommand, OxiPasteError>> {
        match self {
            TextContext::Address(address) => {
                Self::apply_value(config.AddressContextActions.clone(), &address.inner)
            }
            TextContext::Text(text) => {
                Self::apply_value(config.PlainTextContextActions.clone(), text)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ContentTypeId {
    PlainText,
    AddressText,
    Image,
    All,
}

impl Display for ContentTypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text: String = match self {
            ContentTypeId::PlainText => "Text".into(),
            ContentTypeId::AddressText => "Addresses".into(),
            ContentTypeId::Image => "Images".into(),
            ContentTypeId::All => "All".into(),
        };
        write!(f, "{}", text)
    }
}

#[derive(Debug, Clone)]
pub enum ContentType {
    Text(TextContext),
    Image(ImageContext),
}

pub struct GetContextActionsResult(pub Vec<Result<ContextCommand, OxiPasteError>>, pub bool);

impl ContentType {
    pub fn get_view_buttons(
        &self,
        focused_index: usize,
        current_index: usize,
        key: i32,
    ) -> (Button<Message>, Option<Button<Message>>) {
        match self {
            ContentType::Text(context) => {
                context.get_view_buttons(focused_index, current_index, key)
            }
            ContentType::Image(context) => {
                context.get_view_buttons(focused_index, current_index, key)
            }
        }
    }
    pub fn get_context_actions(&self, config: &Config) -> GetContextActionsResult {
        match self {
            ContentType::Text(context) => {
                GetContextActionsResult(context.get_context_actions(config), false)
            }
            ContentType::Image(context) => {
                GetContextActionsResult(context.get_context_actions(config), true)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub(crate) toggled: bool,
    pub(crate) content_type: ContentType,
}

#[derive(Debug, Clone)]
pub enum ContextMenuMessage {
    Expand,
}

#[derive(Debug, Clone)]
pub struct ContextCommand {
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub requires_copy: bool,
}

impl ContextCommand {
    pub fn from_vec_and_value(
        mut args: Vec<String>,
        value: &str,
        requires_copy: bool,
    ) -> Result<Self, OxiPasteError> {
        if args.len() < 2 {
            return Err(OxiPasteError {
                message: "Invalid Command: less than 2 arguments provided".into(),
            });
        }
        let label = args.remove(0);
        let command = args.remove(0);
        let mut found = false;
        if !requires_copy {
            for arg in args.iter_mut() {
                if arg == "{}" {
                    *arg = value.to_owned();
                    found = true;
                }
            }
            if !found {
                args.push(value.to_owned());
            }
        }
        Ok(Self {
            label,
            command,
            args,
            requires_copy,
        })
    }

    pub fn run_command(&self, oxipaste: &OxiPaste, index: i32) -> Result<(), OxiPasteError> {
        if self.requires_copy {
            let res = futures::executor::block_on(copy_to_clipboard(&oxipaste.proxy, index as u32));
            let err_opt = into_general_error(res.err());
            if let Some(error) = err_opt {
                return Err(error);
            }
        }
        let res = Command::new(&self.command).args(&self.args).spawn();
        if let Some(error) = into_general_error(res.err()) {
            Err(error)
        } else {
            Ok(())
        }
    }
}
