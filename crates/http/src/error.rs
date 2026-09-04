use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("rate limited")]
    RateLimited(RateLimited),
    #[error("captcha required")]
    Captcha(CaptchaChallenge),
    #[error("api error")]
    Api(ApiError),
    #[error("transport failure")]
    Transport,
    #[error("response was not valid json")]
    InvalidJson,
    #[error("tls provider failed")]
    TlsProvider,
    #[error("upload cancelled")]
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimited {
    pub retry_after: Duration,
    pub global: bool,
    pub scope: RateLimitScope,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RateLimitScope {
    User,
    Shared,
    Global,
    Unknown,
}

impl RateLimitScope {
    pub fn parse(value: &str) -> Self {
        match value {
            "user" => Self::User,
            "shared" => Self::Shared,
            "global" => Self::Global,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptchaChallenge {
    pub service: CaptchaService,
    pub site_key: Option<String>,
    pub session_id: Option<String>,
    pub rqtoken: Option<String>,
    pub rqdata: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptchaService {
    Hcaptcha,
    Recaptcha,
    Unknown,
}

impl CaptchaService {
    pub fn parse(value: &str) -> Self {
        match value {
            "hcaptcha" => Self::Hcaptcha,
            "recaptcha" => Self::Recaptcha,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiError {
    pub status: u16,
    pub code: ApiErrorCode,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiErrorCode {
    General,
    UnknownAccount,
    UnknownApplication,
    UnknownChannel,
    UnknownGuild,
    UnknownUser,
    UnknownMessage,
    UnknownToken,
    InvalidAuth,
    AccountDisabled,
    AccountScheduledForDeletion,
    MissingAccess,
    MissingPermissions,
    InvalidFormBody,
    InvalidTwoFactor,
    Unknown(u64),
}

impl ApiErrorCode {
    pub fn from_raw(code: u64) -> Self {
        match code {
            0 => Self::General,
            10001 => Self::UnknownAccount,
            10002 => Self::UnknownApplication,
            10003 => Self::UnknownChannel,
            10004 => Self::UnknownGuild,
            10013 => Self::UnknownUser,
            10008 => Self::UnknownMessage,
            10012 => Self::UnknownToken,
            50035 => Self::InvalidFormBody,
            50001 => Self::MissingAccess,
            50013 => Self::MissingPermissions,
            20011 => Self::AccountScheduledForDeletion,
            20013 => Self::AccountDisabled,
            50055 => Self::InvalidAuth,
            60008 => Self::InvalidTwoFactor,
            other => Self::Unknown(other),
        }
    }
}

impl Display for ApiErrorCode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::General => formatter.write_str("general"),
            Self::UnknownAccount => formatter.write_str("unknown account"),
            Self::UnknownApplication => formatter.write_str("unknown application"),
            Self::UnknownChannel => formatter.write_str("unknown channel"),
            Self::UnknownGuild => formatter.write_str("unknown guild"),
            Self::UnknownUser => formatter.write_str("unknown user"),
            Self::UnknownMessage => formatter.write_str("unknown message"),
            Self::UnknownToken => formatter.write_str("unknown token"),
            Self::InvalidAuth => formatter.write_str("invalid auth"),
            Self::AccountDisabled => formatter.write_str("account disabled"),
            Self::AccountScheduledForDeletion => {
                formatter.write_str("account scheduled for deletion")
            }
            Self::MissingAccess => formatter.write_str("missing access"),
            Self::MissingPermissions => formatter.write_str("missing permissions"),
            Self::InvalidFormBody => formatter.write_str("invalid form body"),
            Self::InvalidTwoFactor => formatter.write_str("invalid two factor"),
            Self::Unknown(code) => write!(formatter, "discord {code}"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DiscordErrorBody {
    pub code: Option<u64>,
    pub message: Option<String>,
    pub retry_after: Option<f64>,
    pub global: Option<bool>,
    pub captcha_key: Option<Vec<String>>,
    pub captcha_service: Option<String>,
    pub captcha_sitekey: Option<String>,
    pub captcha_session_id: Option<String>,
    pub captcha_rqtoken: Option<String>,
    pub captcha_rqdata: Option<String>,
}

impl DiscordErrorBody {
    pub fn parse_json(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }

    pub fn into_http_error(self, status: u16, retry_after_header: Option<Duration>) -> HttpError {
        if self.captcha_key.is_some() {
            return HttpError::Captcha(CaptchaChallenge {
                service: self
                    .captcha_service
                    .as_deref()
                    .map(CaptchaService::parse)
                    .unwrap_or(CaptchaService::Unknown),
                site_key: self.captcha_sitekey,
                session_id: self.captcha_session_id,
                rqtoken: self.captcha_rqtoken,
                rqdata: self.captcha_rqdata,
            });
        }
        if status == 429 {
            let seconds = self.retry_after.unwrap_or(1.0);
            let retry_after = retry_after_header.unwrap_or_else(|| seconds_to_duration(seconds));
            return HttpError::RateLimited(RateLimited {
                retry_after,
                global: self.global.unwrap_or(false),
                scope: if self.global.unwrap_or(false) {
                    RateLimitScope::Global
                } else {
                    RateLimitScope::User
                },
            });
        }
        HttpError::Api(ApiError {
            status,
            code: ApiErrorCode::from_raw(self.code.unwrap_or(0)),
            message: self.message.unwrap_or_default(),
        })
    }
}

pub fn seconds_to_duration(seconds: f64) -> Duration {
    if !seconds.is_finite() || seconds <= 0.0 {
        Duration::from_millis(250)
    } else {
        Duration::from_secs_f64(seconds)
    }
}

pub fn parse_retry_after_header(value: &str) -> Option<Duration> {
    value.parse::<f64>().ok().map(seconds_to_duration)
}

#[cfg(test)]
mod tests {
    use super::{ApiErrorCode, DiscordErrorBody, HttpError, RateLimitScope};

    #[test]
    fn captcha_body_becomes_captcha_error() {
        let json = r#"{"captcha_key":["invalid-input-response"],"captcha_service":"hcaptcha","captcha_sitekey":"abc"}"#;
        let body = DiscordErrorBody::parse_json(json.as_bytes());
        let error = body.map(|body| body.into_http_error(400, None));
        assert!(matches!(error, Some(HttpError::Captcha(_))));
    }

    #[test]
    fn rate_limit_body_uses_retry_after_seconds() {
        let json = r#"{"message":"You are being rate limited.","retry_after":1.5,"global":true}"#;
        let body = DiscordErrorBody::parse_json(json.as_bytes()).unwrap();
        let HttpError::RateLimited(limited) = body.into_http_error(429, None) else {
            let matched = false;
            assert!(matched);
            return;
        };
        assert!(limited.global);
        assert_eq!(limited.scope, RateLimitScope::Global);
        assert_eq!(limited.retry_after.as_millis(), 1500);
    }

    #[test]
    fn missing_permissions_maps_typed_code() {
        let json = r#"{"code":50013,"message":"Missing Permissions"}"#;
        let body = DiscordErrorBody::parse_json(json.as_bytes()).unwrap();
        let HttpError::Api(api) = body.into_http_error(403, None) else {
            let matched = false;
            assert!(matched);
            return;
        };
        assert_eq!(api.code, ApiErrorCode::MissingPermissions);
    }
}
