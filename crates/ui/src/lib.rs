use gpui::{Context, IntoElement, Render, Window, div, prelude::*, rgb};

pub struct RusticordRoot;

impl RusticordRoot {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RusticordRoot {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for RusticordRoot {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .bg(rgb(0x111214))
            .text_color(rgb(0xf2f3f5))
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(div().text_xl().child("Rusticord"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xb5bac1))
                            .child("Native client written in Rust"),
                    ),
            )
    }
}
