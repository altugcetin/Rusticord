use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputContentType, InputState};
use gpui_kit::component::{Disableable, v_flex};
use gpui_kit::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Styled, Window, div, px,
};
use rusticord_http::{
    ApiErrorCode, CaptchaSolution, HttpError, LoginCredentials, LoginOutcome, MfaMethod, Password,
    RestClient, runtime_handle,
};
use rusticord_i18n::{Locale, MessageKey, translate};
use rusticord_store::{Settings, SettingsStore, StoredAppearance, StoredLocale, TokenStore};

use crate::palette::{Appearance, AppearancePalette};
use crate::shell::{AuthPhase, MfaPending, Shell};
use crate::theme::to_hsla;

impl Shell {
    pub(crate) fn persist_settings(&self) {
        let Some(store) = self.settings.as_ref() else {
            return;
        };
        let settings = Settings {
            tos_accepted: self.tos_accepted,
            locale: match self.locale {
                Locale::Turkish => StoredLocale::Turkish,
                Locale::English => StoredLocale::English,
            },
            appearance: match self.appearance {
                Appearance::Dark => StoredAppearance::Dark,
                Appearance::Light => StoredAppearance::Light,
            },
            ..Settings::default()
        };
        let _ = store.save(&settings);
    }

    pub(crate) fn set_tos_accepted(&mut self, accepted: bool, cx: &mut Context<Self>) {
        self.tos_accepted = accepted;
        self.persist_settings();
        cx.notify();
    }

    pub(crate) fn submit_login(&mut self, cx: &mut Context<Self>) {
        if !self.tos_accepted || matches!(self.phase, AuthPhase::Busy) {
            return;
        }
        let Some(client) = self.client.clone() else {
            self.error = Some(MessageKey::LoginNetworkError);
            cx.notify();
            return;
        };
        let login = self.login_input.read(cx).value().to_string();
        let password = self.password_input.read(cx).value().to_string();
        let credentials = LoginCredentials {
            login,
            password: Password::new(password),
            undelete: false,
        };
        self.pending_credentials = Some(credentials.clone());
        self.error = None;
        self.phase = AuthPhase::Busy;
        cx.notify();
        spawn_login(client, credentials, None, cx);
    }

    pub(crate) fn submit_mfa(&mut self, cx: &mut Context<Self>) {
        let pending = match &self.phase {
            AuthPhase::AwaitingMfa(pending) => pending.clone(),
            _ => return,
        };
        let Some(client) = self.client.clone() else {
            self.error = Some(MessageKey::LoginNetworkError);
            cx.notify();
            return;
        };
        let code = self.mfa_input.read(cx).value().to_string();
        let captcha = self.captcha_solution(cx);
        self.error = None;
        self.phase = AuthPhase::Busy;
        cx.notify();
        spawn_mfa(client, pending, code, captcha, cx);
    }

    pub(crate) fn submit_captcha(&mut self, cx: &mut Context<Self>) {
        match self.phase {
            AuthPhase::AwaitingCaptcha { resume_mfa: true } => {
                if let Some(pending) = self.pending_mfa_resume.clone() {
                    self.phase = AuthPhase::AwaitingMfa(pending);
                    self.submit_mfa(cx);
                }
            }
            AuthPhase::AwaitingCaptcha { resume_mfa: false } => {
                let Some(client) = self.client.clone() else {
                    self.error = Some(MessageKey::LoginNetworkError);
                    cx.notify();
                    return;
                };
                let Some(credentials) = self.pending_credentials.clone() else {
                    self.error = Some(MessageKey::LoginErrorGeneric);
                    self.phase = AuthPhase::SignedOut;
                    cx.notify();
                    return;
                };
                let captcha = self.captcha_solution(cx);
                self.error = None;
                self.phase = AuthPhase::Busy;
                cx.notify();
                spawn_login(client, credentials, captcha, cx);
            }
            _ => {}
        }
    }

    fn captcha_solution(&self, cx: &App) -> Option<CaptchaSolution> {
        let key = self.captcha_input.read(cx).value().to_string();
        if key.is_empty() {
            return None;
        }
        let challenge = self.pending_captcha.as_ref()?;
        Some(CaptchaSolution {
            key,
            session_id: challenge.session_id.clone(),
            rqtoken: challenge.rqtoken.clone(),
        })
    }

    pub(crate) fn on_login_result(
        &mut self,
        result: Result<LoginOutcome, HttpError>,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(LoginOutcome::Completed { token, .. }) => self.finish_login(token, cx),
            Ok(LoginOutcome::MfaRequired {
                ticket,
                login_instance_id,
                totp,
                sms,
                backup,
                ..
            }) => {
                self.phase = AuthPhase::AwaitingMfa(MfaPending {
                    ticket,
                    login_instance_id,
                    totp,
                    sms,
                    backup,
                });
                self.error = None;
                cx.notify();
            }
            Ok(LoginOutcome::Suspended { .. }) => {
                self.phase = AuthPhase::SignedOut;
                self.error = Some(MessageKey::LoginErrorGeneric);
                cx.notify();
            }
            Err(HttpError::Captcha(challenge)) => {
                self.pending_captcha = Some(challenge);
                self.phase = AuthPhase::AwaitingCaptcha { resume_mfa: false };
                self.error = None;
                cx.notify();
            }
            Err(error) => {
                self.phase = AuthPhase::SignedOut;
                self.error = Some(login_error_key(&error));
                cx.notify();
            }
        }
    }

    pub(crate) fn on_mfa_result(
        &mut self,
        result: Result<String, HttpError>,
        pending: MfaPending,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(token) => self.finish_login(token, cx),
            Err(HttpError::Captcha(challenge)) => {
                self.pending_captcha = Some(challenge);
                self.phase = AuthPhase::AwaitingCaptcha { resume_mfa: true };
                self.pending_mfa_resume = Some(pending);
                self.error = None;
                cx.notify();
            }
            Err(error) => {
                self.phase = AuthPhase::AwaitingMfa(pending);
                self.error = Some(login_error_key(&error));
                cx.notify();
            }
        }
    }

    fn finish_login(&mut self, token: String, cx: &mut Context<Self>) {
        let _ = TokenStore::save(&token);
        if let Some(client) = self.client.clone()
            && let Ok(handle) = runtime_handle()
        {
            drop(handle.spawn(async move {
                client.set_token(Some(token)).await;
            }));
        }
        self.pending_credentials = None;
        self.pending_captcha = None;
        self.pending_mfa_resume = None;
        self.error = None;
        self.phase = AuthPhase::SignedIn;
        cx.notify();
    }

    pub(crate) fn render_signed_out(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = AppearancePalette::for_appearance(self.appearance);
        let locale = self.locale;
        let busy = matches!(self.phase, AuthPhase::Busy);
        let can_login = self.tos_accepted && !busy;

        v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .px_8()
            .child(
                v_flex()
                    .w(px(420.0))
                    .gap_3()
                    .child(
                        div()
                            .text_color(to_hsla(palette.text_primary))
                            .child(translate(locale, MessageKey::RiskNoticeTitle)),
                    )
                    .child(
                        div()
                            .text_color(to_hsla(palette.text_secondary))
                            .child(translate(locale, MessageKey::RiskNoticeBody)),
                    )
                    .child(
                        Checkbox::new("tos-accept")
                            .label(translate(locale, MessageKey::RiskNoticeAccept))
                            .checked(self.tos_accepted)
                            .disabled(busy)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.set_tos_accepted(*checked, cx);
                            })),
                    )
                    .child(self.render_auth_fields(palette, locale, can_login, busy, cx))
                    .child(self.render_error(palette, locale)),
            )
    }

    fn render_auth_fields(
        &self,
        palette: AppearancePalette,
        locale: Locale,
        can_login: bool,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        match &self.phase {
            AuthPhase::AwaitingMfa(_) => self
                .render_mfa_fields(palette, locale, busy, cx)
                .into_any_element(),
            AuthPhase::AwaitingCaptcha { .. } => self
                .render_captcha_fields(palette, locale, can_login, busy, cx)
                .into_any_element(),
            AuthPhase::SignedOut | AuthPhase::Busy => self
                .render_password_fields(palette, locale, can_login, busy, cx)
                .into_any_element(),
            AuthPhase::SignedIn => div().into_any_element(),
        }
    }

    fn render_password_fields(
        &self,
        palette: AppearancePalette,
        locale: Locale,
        can_login: bool,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                div()
                    .text_color(to_hsla(palette.text_primary))
                    .child(translate(locale, MessageKey::LoginTitle)),
            )
            .child(
                Input::new(&self.login_input)
                    .w_full()
                    .disabled(!can_login)
                    .content_type(InputContentType::Username)
                    .aria_label(translate(locale, MessageKey::LoginIdentifier)),
            )
            .child(
                Input::new(&self.password_input)
                    .w_full()
                    .disabled(!can_login)
                    .mask_toggle()
                    .content_type(InputContentType::Password)
                    .aria_label(translate(locale, MessageKey::LoginPassword)),
            )
            .child(
                Button::new("login-submit")
                    .primary()
                    .label(if busy {
                        translate(locale, MessageKey::LoginBusy)
                    } else {
                        translate(locale, MessageKey::LoginSubmit)
                    })
                    .disabled(!can_login)
                    .on_click(cx.listener(|this, _, _, cx| this.submit_login(cx))),
            )
    }

    fn render_mfa_fields(
        &self,
        palette: AppearancePalette,
        locale: Locale,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                div()
                    .text_color(to_hsla(palette.text_primary))
                    .child(translate(locale, MessageKey::MfaTitle)),
            )
            .child(
                Input::new(&self.mfa_input)
                    .w_full()
                    .disabled(busy)
                    .content_type(InputContentType::OneTimeCode)
                    .aria_label(translate(locale, MessageKey::MfaCode)),
            )
            .child(
                Button::new("mfa-submit")
                    .primary()
                    .label(if busy {
                        translate(locale, MessageKey::LoginBusy)
                    } else {
                        translate(locale, MessageKey::MfaSubmit)
                    })
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, _, cx| this.submit_mfa(cx))),
            )
    }

    fn render_captcha_fields(
        &self,
        palette: AppearancePalette,
        locale: Locale,
        can_login: bool,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                div()
                    .text_color(to_hsla(palette.text_primary))
                    .child(translate(locale, MessageKey::CaptchaTitle)),
            )
            .child(
                div()
                    .text_color(to_hsla(palette.text_secondary))
                    .child(translate(locale, MessageKey::CaptchaBody)),
            )
            .child(
                Input::new(&self.captcha_input)
                    .w_full()
                    .disabled(busy)
                    .aria_label(translate(locale, MessageKey::CaptchaKey)),
            )
            .child(
                Button::new("captcha-submit")
                    .primary()
                    .label(if busy {
                        translate(locale, MessageKey::LoginBusy)
                    } else {
                        translate(locale, MessageKey::CaptchaSubmit)
                    })
                    .disabled(!can_login)
                    .on_click(cx.listener(|this, _, _, cx| this.submit_captcha(cx))),
            )
    }

    fn render_error(&self, palette: AppearancePalette, locale: Locale) -> impl IntoElement {
        if let Some(key) = self.error {
            div()
                .text_color(to_hsla(palette.danger))
                .child(translate(locale, key))
        } else {
            div()
        }
    }
}

fn login_error_key(error: &HttpError) -> MessageKey {
    match error {
        HttpError::Transport => MessageKey::LoginNetworkError,
        HttpError::Api(api) if api.code == ApiErrorCode::InvalidAuth => {
            MessageKey::LoginInvalidAuth
        }
        _ => MessageKey::LoginErrorGeneric,
    }
}

fn spawn_login(
    client: Arc<RestClient>,
    credentials: LoginCredentials,
    captcha: Option<CaptchaSolution>,
    cx: &mut Context<Shell>,
) {
    let Ok(handle) = runtime_handle() else {
        return;
    };
    let (tx, rx) = tokio::sync::oneshot::channel();
    drop(handle.spawn(async move {
        let result = client.login(&credentials, captcha.as_ref()).await;
        let _ = tx.send(result);
    }));
    cx.spawn(async move |this, cx| match rx.await {
        Ok(result) => {
            let _ = this.update(cx, |this, cx| this.on_login_result(result, cx));
        }
        Err(_) => {
            let _ = this.update(cx, |this, cx| {
                this.phase = AuthPhase::SignedOut;
                this.error = Some(MessageKey::LoginErrorGeneric);
                cx.notify();
            });
        }
    })
    .detach();
}

fn spawn_mfa(
    client: Arc<RestClient>,
    pending: MfaPending,
    code: String,
    captcha: Option<CaptchaSolution>,
    cx: &mut Context<Shell>,
) {
    let Ok(handle) = runtime_handle() else {
        return;
    };
    let (tx, rx) = tokio::sync::oneshot::channel();
    let ticket = pending.ticket.clone();
    let login_instance_id = pending.login_instance_id.clone();
    let method = if pending.totp {
        MfaMethod::Totp
    } else if pending.sms {
        MfaMethod::Sms
    } else if pending.backup {
        MfaMethod::Backup
    } else {
        MfaMethod::Webauthn
    };
    drop(handle.spawn(async move {
        let result = client
            .verify_mfa(
                method,
                &ticket,
                &code,
                login_instance_id.as_deref(),
                captcha.as_ref(),
            )
            .await
            .map(|body| body.token);
        let _ = tx.send(result);
    }));
    cx.spawn(async move |this, cx| match rx.await {
        Ok(result) => {
            let _ = this.update(cx, |this, cx| this.on_mfa_result(result, pending, cx));
        }
        Err(_) => {
            let _ = this.update(cx, |this, cx| {
                this.phase = AuthPhase::AwaitingMfa(pending);
                this.error = Some(MessageKey::LoginErrorGeneric);
                cx.notify();
            });
        }
    })
    .detach();
}

pub(crate) fn restore_session(tos_accepted: bool, client: &Option<Arc<RestClient>>) -> AuthPhase {
    if !tos_accepted {
        return AuthPhase::SignedOut;
    }
    let Ok(Some(token)) = TokenStore::load() else {
        return AuthPhase::SignedOut;
    };
    if let Some(client) = client.clone()
        && let Ok(handle) = runtime_handle()
    {
        let value = String::from(token.as_str());
        drop(handle.spawn(async move {
            client.set_token(Some(value)).await;
        }));
    }
    AuthPhase::SignedIn
}

pub(crate) fn make_input(
    window: &mut Window,
    cx: &mut Context<Shell>,
    placeholder: &str,
    masked: bool,
) -> Entity<InputState> {
    let placeholder = String::from(placeholder);
    cx.new(|cx| {
        let mut state = InputState::new(window, cx).placeholder(placeholder);
        if masked {
            state = state.masked(true);
        }
        state
    })
}

pub(crate) fn load_settings() -> (bool, Locale, Appearance, Option<SettingsStore>) {
    let store = SettingsStore::open_default().ok();
    let settings = store
        .as_ref()
        .and_then(|store| store.load().ok())
        .unwrap_or_default();
    let locale = match settings.locale {
        StoredLocale::Turkish => Locale::Turkish,
        StoredLocale::English => Locale::English,
    };
    let appearance = match settings.appearance {
        StoredAppearance::Dark => Appearance::Dark,
        StoredAppearance::Light => Appearance::Light,
    };
    (settings.tos_accepted, locale, appearance, store)
}
