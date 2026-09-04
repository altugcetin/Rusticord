use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::InputState;
use gpui_kit::component::{TitleBar, h_flex, v_flex};
use gpui_kit::{
    App, Bounds, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, WindowBounds,
    WindowOptions, div, px, size,
};
use rusticord_http::{CaptchaChallenge, LoginCredentials, RestClient};
use rusticord_i18n::{Locale, MessageKey, translate};
use rusticord_platform::APPLICATION_IDENTIFIER;
use rusticord_store::SettingsStore;

use crate::login::{load_settings, make_input, restore_session};
use crate::palette::{
    Appearance, AppearancePalette, CHANNEL_SIDEBAR_WIDTH, GUILD_RAIL_WIDTH, MEMBER_LIST_WIDTH,
};
use crate::theme::{apply_appearance, to_hsla};

#[derive(Clone, Debug)]
pub(crate) struct MfaPending {
    pub ticket: String,
    pub login_instance_id: Option<String>,
    pub totp: bool,
    pub sms: bool,
    pub backup: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum AuthPhase {
    SignedOut,
    Busy,
    AwaitingMfa(MfaPending),
    AwaitingCaptcha { resume_mfa: bool },
    SignedIn,
}

pub struct Shell {
    pub(crate) appearance: Appearance,
    pub(crate) locale: Locale,
    pub(crate) tos_accepted: bool,
    pub(crate) phase: AuthPhase,
    pub(crate) error: Option<MessageKey>,
    pub(crate) login_input: Entity<InputState>,
    pub(crate) password_input: Entity<InputState>,
    pub(crate) mfa_input: Entity<InputState>,
    pub(crate) captcha_input: Entity<InputState>,
    pub(crate) client: Option<Arc<RestClient>>,
    pub(crate) settings: Option<SettingsStore>,
    pub(crate) pending_credentials: Option<LoginCredentials>,
    pub(crate) pending_captcha: Option<CaptchaChallenge>,
    pub(crate) pending_mfa_resume: Option<MfaPending>,
}

impl Shell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (tos_accepted, locale, appearance, settings) = load_settings();
        apply_appearance(appearance, None, cx);
        let client = RestClient::new().ok().map(Arc::new);
        let login_input = make_input(
            window,
            cx,
            translate(locale, MessageKey::LoginIdentifier),
            false,
        );
        let password_input = make_input(
            window,
            cx,
            translate(locale, MessageKey::LoginPassword),
            true,
        );
        let mfa_input = make_input(window, cx, translate(locale, MessageKey::MfaCode), false);
        let captcha_input =
            make_input(window, cx, translate(locale, MessageKey::CaptchaKey), false);
        let phase = restore_session(tos_accepted, &client);
        Self {
            appearance,
            locale,
            tos_accepted,
            phase,
            error: None,
            login_input,
            password_input,
            mfa_input,
            captcha_input,
            client,
            settings,
            pending_credentials: None,
            pending_captcha: None,
            pending_mfa_resume: None,
        }
    }

    fn toggle_appearance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.appearance = match self.appearance {
            Appearance::Dark => Appearance::Light,
            Appearance::Light => Appearance::Dark,
        };
        apply_appearance(self.appearance, Some(window), cx);
        self.persist_settings();
        cx.notify();
    }
}

pub fn application_window_options(cx: &App) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        app_id: Some(String::from(APPLICATION_IDENTIFIER)),
        window_min_size: Some(size(px(700.0), px(480.0))),
        ..TitleBar::window_options()
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = AppearancePalette::for_appearance(self.appearance);
        let locale = self.locale;
        let appearance_label = match self.appearance {
            Appearance::Dark => translate(locale, MessageKey::AppearanceLight),
            Appearance::Light => translate(locale, MessageKey::AppearanceDark),
        };
        let signed_in = matches!(self.phase, AuthPhase::SignedIn);

        v_flex()
            .size_full()
            .bg(to_hsla(palette.bg_base))
            .text_color(to_hsla(palette.text_primary))
            .child(
                TitleBar::new().child(
                    h_flex()
                        .w_full()
                        .h_full()
                        .items_center()
                        .justify_between()
                        .pr_2()
                        .child(translate(locale, MessageKey::ApplicationName))
                        .child(
                            Button::new("toggle-appearance")
                                .ghost()
                                .label(appearance_label)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.toggle_appearance(window, cx);
                                })),
                        ),
                ),
            )
            .child(if signed_in {
                signed_in_workspace(palette, locale).into_any_element()
            } else {
                self.render_signed_out(cx).into_any_element()
            })
    }
}

fn signed_in_workspace(palette: AppearancePalette, locale: Locale) -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .min_h_0()
        .child(guild_rail(palette, locale))
        .child(channel_sidebar(palette, locale))
        .child(chat_pane(palette, locale))
        .child(member_list(palette, locale))
}

fn guild_rail(palette: AppearancePalette, locale: Locale) -> impl IntoElement {
    v_flex()
        .w(px(GUILD_RAIL_WIDTH))
        .h_full()
        .bg(to_hsla(palette.bg_surface))
        .border_r_1()
        .border_color(to_hsla(palette.border_subtle))
        .child(empty_copy(
            palette,
            translate(locale, MessageKey::EmptyGuildsTitle),
            translate(locale, MessageKey::EmptyGuildsBody),
        ))
}

fn channel_sidebar(palette: AppearancePalette, locale: Locale) -> impl IntoElement {
    v_flex()
        .w(px(CHANNEL_SIDEBAR_WIDTH))
        .h_full()
        .bg(to_hsla(palette.bg_surface))
        .border_r_1()
        .border_color(to_hsla(palette.border_subtle))
        .child(pane_heading(
            palette,
            translate(locale, MessageKey::ChannelSidebarTitle),
        ))
        .child(empty_copy(
            palette,
            translate(locale, MessageKey::EmptyChannelsTitle),
            translate(locale, MessageKey::EmptyChannelsBody),
        ))
}

fn chat_pane(palette: AppearancePalette, locale: Locale) -> impl IntoElement {
    v_flex()
        .flex_1()
        .h_full()
        .bg(to_hsla(palette.bg_base))
        .child(pane_heading(
            palette,
            translate(locale, MessageKey::ChatHeaderPlaceholder),
        ))
        .child(empty_copy(
            palette,
            translate(locale, MessageKey::EmptyChatTitle),
            translate(locale, MessageKey::EmptyChatBody),
        ))
}

fn member_list(palette: AppearancePalette, locale: Locale) -> impl IntoElement {
    v_flex()
        .w(px(MEMBER_LIST_WIDTH))
        .h_full()
        .bg(to_hsla(palette.bg_surface))
        .border_l_1()
        .border_color(to_hsla(palette.border_subtle))
        .child(pane_heading(
            palette,
            translate(locale, MessageKey::MemberListTitle),
        ))
        .child(empty_copy(
            palette,
            translate(locale, MessageKey::EmptyMembersTitle),
            translate(locale, MessageKey::EmptyMembersBody),
        ))
}

fn pane_heading(palette: AppearancePalette, title: &'static str) -> impl IntoElement {
    div()
        .h(px(48.0))
        .px_4()
        .flex()
        .items_center()
        .border_b_1()
        .border_color(to_hsla(palette.border_subtle))
        .text_color(to_hsla(palette.text_secondary))
        .child(title)
}

fn empty_copy(
    palette: AppearancePalette,
    title: &'static str,
    body: &'static str,
) -> impl IntoElement {
    v_flex()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_2()
        .px_4()
        .child(div().text_color(to_hsla(palette.text_primary)).child(title))
        .child(div().text_color(to_hsla(palette.text_muted)).child(body))
}
