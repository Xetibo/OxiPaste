use std::fmt::Display;
use std::path::PathBuf;

use config::{Config, create_config, default_config, parse_config};
use context::{
    Address, ContentType, ContentTypeId, ContextCommand, ContextMenu, ContextMenuMessage,
    GetContextActionsResult, ImageContext, TextContext,
};
use iced::keyboard::Modifiers;
use iced::keyboard::key::Named;
use iced::widget::{Column, Row, column, row, scrollable};
use iced::{Alignment, Color, Element, Length, Task, Theme, event, futures};
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use oxiced::theme::theme::get_derived_iced_theme;
use oxiced::widgets::oxi_button::{ButtonVariant, button};
use oxiced::widgets::oxi_layer::{layer_theme, rounded_layer};
use oxiced::widgets::oxi_picklist::pick_list;
use oxiced::widgets::oxi_svg::{self, SvgStyleVariant};
use oxiced::widgets::oxi_text_input::text_input;

use iced_layershell::Application;
use iced_layershell::actions::LayershellCustomActions;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use utils::{FocusDirection, mk_svg, svg_path};
use zbus::{Connection, proxy};

mod config;
mod context;
mod custom_rich;
mod utils;

const SVG_PATH: Lazy<PathBuf> = Lazy::new(|| svg_path());
const WINDOW_SIZE: (u32, u32) = (600, 800);
const WINDOW_MARGIN: (i32, i32, i32, i32) = (100, 100, 100, 100);

pub fn main() -> Result<(), iced_layershell::Error> {
    let settings = Settings {
        layer_settings: LayerShellSettings {
            size: Some(WINDOW_SIZE),
            exclusive_zone: 0,
            anchor: Anchor::Left | Anchor::Right | Anchor::Top | Anchor::Bottom,
            layer: Layer::Overlay,
            margin: WINDOW_MARGIN,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        },
        ..Default::default()
    };
    OxiPaste::run(settings)
}

#[derive(Debug, Clone)]
pub struct OxiPasteError {
    message: String,
}

impl Display for OxiPasteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl OxiPasteError {
    fn new(message: &'static str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

impl std::error::Error for OxiPasteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        &self.message
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        None
    }
}

pub fn into_general_error(
    error_opt: Option<impl std::error::Error + 'static>,
) -> Option<OxiPasteError> {
    let error = error_opt?;
    Some(OxiPasteError {
        message: error.to_string(),
    })
}

struct OxiPaste {
    theme: Theme,
    filter_text: String,
    filter_content_type: ContentTypeId,
    filtered_content: Vec<(i32, ContextMenu)>,
    clipboard_content: IndexMap<i32, ContextMenu>,
    proxy: OxiPasteDbusProxy<'static>,
    errors: Vec<OxiPasteError>,
    config: Config,
    focus: usize,
}

impl Default for OxiPaste {
    fn default() -> Self {
        // when we don't have a proxy, we have other issues, aka goodbye
        let proxy = futures::executor::block_on(get_connection()).unwrap();
        let data = futures::executor::block_on(get_items(&proxy));
        let mut errors = Vec::new();
        let (clipboard_content, error_opt) = if let Ok(map) = data {
            (map, None)
        } else {
            (IndexMap::new(), into_general_error(data.err()))
        };
        error_opt.into_iter().for_each(|value| errors.push(value));
        let config_dir = create_config();
        let (config, error_opt) = if let Ok(dir) = config_dir {
            let config_res = parse_config(&dir);
            if let Ok(config) = config_res {
                (config, None)
            } else {
                (default_config(), config_res.unwrap_err())
            }
        } else {
            (default_config(), config_dir.unwrap_err())
        };
        error_opt.into_iter().for_each(|value| errors.push(value));
        let filtered_content = clipboard_content
            .iter()
            .map(|(key, value)| (*key, value.clone()))
            .collect::<Vec<_>>();
        Self {
            theme: get_derived_iced_theme(),
            filter_text: "".into(),
            filter_content_type: ContentTypeId::All,
            filtered_content,
            clipboard_content,
            proxy,
            errors,
            config,
            focus: 0,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Copy(i32),
    CopyFromKeyboard(i32),
    Remove(i32),
    ClearClipboard,
    SetFilterText(String),
    SetContentTypeFilter(ContentTypeId),
    RunContextCommand(ContextCommand, bool, i32),
    SubMessageContext(i32, ContextMenuMessage),
    MoveFocus(FocusDirection),
    Exit,
    Enter,
}

impl TryInto<LayershellCustomActions> for Message {
    type Error = Self;
    fn try_into(self) -> Result<LayershellCustomActions, Self::Error> {
        Err(self)
    }
}

impl OxiPaste {
    fn copy(&mut self, index: u32) -> Task<Message> {
        let res = futures::executor::block_on(copy_to_clipboard(&self.proxy, index));
        into_general_error(res.err())
            .into_iter()
            .for_each(|value| self.errors.push(value));
        // TODO make this work with iced exit?
        exit(&self.config);
        Task::none()
    }

    fn filter(&mut self) {
        // Reset the focus on filter
        self.focus = 0;
        self.filtered_content = self
            .clipboard_content
            .iter()
            .filter(|(_, value)| match &value.content_type {
                ContentType::Text(text_content) => {
                    let (text, allow_type) = match text_content {
                        TextContext::Text(text) => (
                            text,
                            (self.filter_content_type == ContentTypeId::All
                                || self.filter_content_type == ContentTypeId::PlainText),
                        ),
                        TextContext::Address(address) => (
                            &address.inner,
                            (self.filter_content_type == ContentTypeId::All
                                || self.filter_content_type == ContentTypeId::AddressText),
                        ),
                    };
                    text.to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
                        && allow_type
                }
                ContentType::Image(_) => {
                    (self.filter_text.contains("image") || self.filter_text.is_empty())
                        && (self.filter_content_type == ContentTypeId::All
                            || self.filter_content_type == ContentTypeId::Image)
                }
            })
            .map(|(key, value)| (*key, value.clone()))
            .collect::<Vec<(i32, ContextMenu)>>();
    }
}

impl Application for OxiPaste {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(_flags: ()) -> (Self, Task<Message>) {
        (
            Self {
                ..Default::default()
            },
            iced::widget::text_input::focus("search_box"),
        )
    }

    fn namespace(&self) -> String {
        String::from("OxiPaste")
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        event::listen_with(|event, _status, _id| match event {
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                modifiers: modifier,
                key: iced::keyboard::key::Key::Named(key),
                modified_key: _,
                physical_key: _,
                location: _,
                text: _,
            }) => match key {
                Named::Escape => Some(Message::Exit),
                Named::Enter => Some(Message::Enter),
                Named::Tab => match modifier {
                    Modifiers::SHIFT => Some(Message::MoveFocus(FocusDirection::Up)),
                    _ => Some(Message::MoveFocus(FocusDirection::Down)),
                },
                _ => None,
            },
            _ => None,
        })
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        self.errors.clear();
        match message {
            Message::Copy(value) => self.copy(value as u32),
            Message::CopyFromKeyboard(value) => {
                let converted_value_res = self.filtered_content.get(value as usize);
                if let Some(converted_value) = converted_value_res {
                    return self.copy(converted_value.0 as u32);
                }
                self.errors.push(OxiPasteError {
                    message: "Could not get indexed value".into(),
                });
                Task::none()
            }
            Message::SetFilterText(value) => {
                self.filter_text = value;
                self.filter();
                Task::none()
            }
            Message::SetContentTypeFilter(value) => {
                self.filter_content_type = value;
                self.filter();
                Task::none()
            }
            Message::Remove(index) => {
                self.clipboard_content.shift_remove(&index);
                self.filter();
                Task::none()
            }
            Message::ClearClipboard => {
                let res = futures::executor::block_on(delete_all(&self.proxy));
                into_general_error(res.err())
                    .into_iter()
                    .for_each(|value| self.errors.push(value));
                // TODO make this work with iced exit?
                exit(&self.config);
                Task::none()
            }
            Message::RunContextCommand(command, copy, index) => {
                if copy {
                    let res =
                        futures::executor::block_on(copy_to_clipboard(&self.proxy, index as u32));
                    into_general_error(res.err())
                        .into_iter()
                        .for_each(|value| self.errors.push(value));
                }
                let res = command.run_command(self, index);
                into_general_error(res.err())
                    .into_iter()
                    .for_each(|value| self.errors.push(value));
                exit(&self.config);
                Task::none()
            }
            Message::SubMessageContext(index, ContextMenuMessage::Expand) => {
                let context = self.clipboard_content.get_mut(&index).unwrap();
                context.toggled = !context.toggled;
                self.filter();
                Task::none()
            }
            Message::Exit => {
                // TODO make this work with iced exit?
                std::process::exit(0);
            }
            Message::MoveFocus(focus_direction) => {
                self.focus = focus_direction.add(self.focus, self.filtered_content.len());
                // TODO use when accessiblity is not complete ass
                //iced::widget::focus_next()
                Task::none()
            }
            Message::Enter => Task::done(Message::CopyFromKeyboard(self.focus as i32)),
        }
    }

    fn view(&self) -> Element<Message> {
        rounded_layer(window(self), WINDOW_SIZE)
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    // remove the annoying background color
    fn style(&self, _: &Self::Theme) -> iced_layershell::Appearance {
        layer_theme()
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }
}

fn error_view(error: OxiPasteError) -> Row<'static, Message> {
    let text: String = error.to_string();
    row![iced::widget::text(format!("Error: {}", text)).color(Color::from_rgb(1.0, 0.0, 0.0))]
}

fn clipboard_element<'a>(
    index: usize,
    key: i32,
    context: &'a ContextMenu,
    state: &'a OxiPaste,
) -> Row<'a, Message> {
    let (content_button, context_button) =
        context
            .content_type
            .get_view_buttons(state.focus, index, key);
    if context.toggled {
        // TODO rework this copy
        let GetContextActionsResult(choices, copy) =
            context.content_type.get_context_actions(&state.config);
        // TODO make this error do shit
        row![
            Row::with_children(choices.into_iter().map(|choice| {
                if choice.is_err() {
                    error_view(choice.err().unwrap()).into()
                } else {
                    let command = choice.unwrap();
                    let mut label = command.label.clone();
                    label.truncate(5);

                    button(iced::widget::text(label), ButtonVariant::Neutral)
                        .on_press(Message::RunContextCommand(command, copy, key))
                        .into()
                }
            }))
            .spacing(20)
            .width(iced::Length::Fill),
            button(
                oxi_svg::svg_from_path(SvgStyleVariant::Primary, mk_svg("delete.svg")),
                ButtonVariant::Neutral
            )
            .on_press(Message::Remove(key))
            .width(45)
            .height(45),
            context_button.unwrap()
        ]
        .padding(20)
        .align_y(Alignment::Center)
        .spacing(20)
    } else {
        row![
            content_button
                .width(iced::Length::Fill)
                .on_press(Message::Copy(key)),
            button(
                oxi_svg::svg_from_path(SvgStyleVariant::Primary, mk_svg("delete.svg")),
                ButtonVariant::Neutral
            )
            .on_press(Message::Remove(key))
            .width(45)
            .height(45),
            if context_button.is_some() {
                row![context_button.unwrap()]
            } else {
                row![]
            },
        ]
        .padding(20)
        .align_y(Alignment::Center)
        .spacing(20)
    }
}

fn window(state: &OxiPaste) -> Column<Message> {
    let elements: Vec<Row<'_, Message>> = state
        .filtered_content
        .iter()
        .enumerate()
        .map(|(index, (key, value))| clipboard_element(index, *key, value, state))
        .collect();

    let mut elements_col = column![];
    for element in elements {
        elements_col = elements_col.push_maybe(Some(element));
    }
    let elements_scrollable = scrollable(elements_col);

    let mut col = Column::new()
        .push(
            column![
                row![
                    pick_list(
                        [
                            ContentTypeId::All,
                            ContentTypeId::PlainText,
                            ContentTypeId::AddressText,
                            ContentTypeId::Image
                        ],
                        Some(state.filter_content_type),
                        Message::SetContentTypeFilter
                    )
                    .width(Length::Fill),
                    button("Clear all", ButtonVariant::Neutral).on_press(Message::ClearClipboard)
                ]
                .spacing(10),
                text_input(
                    "Enter text to find",
                    state.filter_text.as_str(),
                    Message::SetFilterText
                )
                .id("search_box"),
            ]
            .padding(20)
            .spacing(20),
        )
        .push(elements_scrollable)
        .padding(10)
        .spacing(20)
        .align_x(Alignment::Center);

    let error_views = state.errors.clone().into_iter().map(error_view);

    for error in error_views {
        col = col.push(error);
    }
    col
}

#[proxy(
    interface = "org.Xetibo.OxiPasteDaemon",
    default_service = "org.Xetibo.OxiPasteDaemon",
    default_path = "/org/Xetibo/OxiPasteDaemon"
)]
#[allow(non_snake_case)]
trait OxiPasteDbus {
    async fn GetAll(&self) -> zbus::Result<Vec<(Vec<u8>, String)>>;
    async fn Paste(&self, index: u32) -> zbus::Result<()>;
    async fn DeleteAll(&self) -> zbus::Result<()>;
}

async fn get_items(proxy: &OxiPasteDbusProxy<'static>) -> zbus::Result<IndexMap<i32, ContextMenu>> {
    let reply = proxy.GetAll().await?;

    let mut map = IndexMap::new();
    for item in reply {
        if item.1.contains("text/plain") {
            let item_res = String::from_utf8(item.0);
            if item_res.is_err() {
                return Err(zbus::Error::Failure(
                    "Could not convert data from daemon".into(),
                ));
            }
            let address_opt = Address::try_build(item_res.unwrap());
            map.insert(
                map.len() as i32,
                ContextMenu {
                    toggled: false,
                    content_type: if let Ok(address) = address_opt {
                        ContentType::Text(TextContext::Address(address))
                    } else {
                        // guaranteed error -> aka text, lmao
                        ContentType::Text(TextContext::Text(address_opt.unwrap_err()))
                    },
                },
            );
        } else {
            map.insert(
                map.len() as i32,
                ContextMenu {
                    toggled: false,
                    content_type: ContentType::Image(ImageContext::Regular(item.0)),
                },
            );
        }
    }
    Ok(map)
}

async fn copy_to_clipboard(proxy: &OxiPasteDbusProxy<'static>, index: u32) -> zbus::Result<()> {
    proxy.Paste(index).await?;
    Ok(())
}

async fn delete_all(proxy: &OxiPasteDbusProxy<'static>) -> zbus::Result<()> {
    proxy.DeleteAll().await?;
    Ok(())
}

async fn get_connection() -> zbus::Result<OxiPasteDbusProxy<'static>> {
    let connection = Connection::session().await?;
    let proxy = OxiPasteDbusProxy::new(&connection).await?;
    Ok(proxy)
}

fn exit(config: &Config) {
    if !config.keepOpen {
        std::process::exit(0);
    }
}
