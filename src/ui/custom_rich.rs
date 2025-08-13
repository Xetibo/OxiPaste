use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget};
use iced::widget::text::{Catalog, Rich};
use iced::{Length, Rectangle, Size};

pub struct CustomRich<'a, Link, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Link: Clone + 'static,
    Theme: Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    inner: Rich<'a, Link, Message, Theme, Renderer>,
}

#[allow(clippy::self_named_constructors)]
impl<'a, Link, Message, Theme, Renderer> CustomRich<'a, Link, Message, Theme, Renderer>
where
    Link: Clone + 'static,
    Theme: Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    pub fn new(inner: Rich<'a, Link, Message, Theme, Renderer>) -> Self {
        Self { inner }
    }
    pub fn custom_rich(inner: Rich<'a, Link, Message, Theme, Renderer>) -> Self {
        Self::new(inner)
    }
}

impl<'a, Link, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for CustomRich<'a, Link, Message, Theme, Renderer>
where
    Link: Clone + 'static,
    Theme: Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    fn size_hint(&self) -> Size<Length> {
        self.inner.size_hint()
    }

    fn tag(&self) -> widget::tree::Tag {
        self.inner.tag()
    }

    fn state(&self) -> widget::tree::State {
        self.inner.state()
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.inner.children()
    }

    fn operate(
        &self,
        state: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.inner.operate(state, layout, renderer, operation)
    }

    fn mouse_interaction(
        &self,
        _state: &widget::Tree,
        _layout: Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        iced::advanced::mouse::Interaction::None
    }

    fn size(&self) -> Size<Length> {
        iced::Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.inner.layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.inner
            .draw(tree, renderer, theme, style, layout, cursor, viewport)
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.children.clear();
    }

    fn update(
        &mut self,
        _state: &mut widget::Tree,
        _event: &iced::Event,
        _layout: Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        _shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
    }
}

impl<'a, Link, Message, Theme, Renderer> From<CustomRich<'a, Link, Message, Theme, Renderer>>
    for iced::Element<'a, Message, Theme, Renderer>
where
    Link: Clone + 'static,
    Theme: Catalog + 'a,
    Renderer: iced::advanced::text::Renderer + 'a,
    Message: Clone + 'a,
{
    fn from(rich: CustomRich<'a, Link, Message, Theme, Renderer>) -> Self {
        Self::new(rich)
    }
}
