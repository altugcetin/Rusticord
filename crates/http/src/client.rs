use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use reqwest::header::HeaderMap;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::auth::{
    EncryptedTokenBody, LoginBody, LoginCredentials, LoginOutcome, LoginSuccessBody, MfaBody,
    MfaMethod, MfaSmsSendBody, MfaSuccessBody, RemoteAuthLoginBody,
};
use crate::error::{ApiError, ApiErrorCode, DiscordErrorBody, HttpError, RateLimited};
use crate::rate_limit::{RateLimitHeaders, RateLimiter, retry_wait};
use crate::route::{RestRoute, rest_url};
use crate::upload::{
    CancelFlag, ProgressBody, UploadFile, UploadProgress, encode_multipart, make_boundary,
    multipart_content_type,
};

const USER_AGENT: &str = "Rusticord (ist.alchm.rusticord, 0.1.0)";
const MAX_RETRY_ATTEMPTS: u8 = 5;

pub struct RestClient {
    http: reqwest::Client,
    limiter: Mutex<RateLimiter>,
    token: Mutex<Option<String>>,
}

#[derive(Clone, Debug, Default)]
pub struct CaptchaSolution {
    pub key: String,
    pub session_id: Option<String>,
    pub rqtoken: Option<String>,
}

enum Outgoing {
    Empty,
    Json(Vec<u8>),
    Multipart {
        content_type: String,
        body: Vec<u8>,
        on_progress: Option<Arc<dyn Fn(UploadProgress) + Send + Sync>>,
        cancel: Option<CancelFlag>,
    },
}

impl RestClient {
    pub fn new() -> Result<Self, HttpError> {
        let _ = rustls_graviola::default_provider().install_default();
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .map_err(|_| HttpError::Transport)?;
        Ok(Self {
            http,
            limiter: Mutex::new(RateLimiter::new(Instant::now())),
            token: Mutex::new(None),
        })
    }

    pub async fn set_token(&self, token: Option<String>) {
        *self.token.lock().await = token;
    }

    pub async fn token(&self) -> Option<String> {
        self.token.lock().await.clone()
    }

    pub async fn login(
        &self,
        credentials: &LoginCredentials,
        captcha: Option<&CaptchaSolution>,
    ) -> Result<LoginOutcome, HttpError> {
        let body = LoginBody {
            login: &credentials.login,
            password: credentials.password.as_str(),
            undelete: credentials.undelete,
        };
        let bytes = self
            .dispatch(&RestRoute::login(), Outgoing::json(&body)?, captcha, false)
            .await?;
        let parsed: LoginSuccessBody =
            serde_json::from_slice(&bytes).map_err(|_| HttpError::InvalidJson)?;
        parsed.into_outcome().ok_or(HttpError::InvalidJson)
    }

    pub async fn verify_mfa(
        &self,
        method: MfaMethod,
        ticket: &str,
        code: &str,
        login_instance_id: Option<&str>,
        captcha: Option<&CaptchaSolution>,
    ) -> Result<MfaSuccessBody, HttpError> {
        let body = MfaBody {
            ticket,
            code,
            login_instance_id,
        };
        let bytes = self
            .dispatch(&method.route(), Outgoing::json(&body)?, captcha, false)
            .await?;
        serde_json::from_slice(&bytes).map_err(|_| HttpError::InvalidJson)
    }

    pub async fn send_mfa_sms(
        &self,
        ticket: &str,
        captcha: Option<&CaptchaSolution>,
    ) -> Result<(), HttpError> {
        let body = MfaSmsSendBody { ticket };
        let _ = self
            .dispatch(
                &RestRoute::mfa_sms_send(),
                Outgoing::json(&body)?,
                captcha,
                false,
            )
            .await?;
        Ok(())
    }

    pub async fn exchange_remote_auth_ticket(
        &self,
        ticket: &str,
        captcha: Option<&CaptchaSolution>,
    ) -> Result<EncryptedTokenBody, HttpError> {
        let body = RemoteAuthLoginBody { ticket };
        let bytes = self
            .dispatch(
                &RestRoute::remote_auth_login(),
                Outgoing::json(&body)?,
                captcha,
                false,
            )
            .await?;
        serde_json::from_slice(&bytes).map_err(|_| HttpError::InvalidJson)
    }

    pub async fn current_user_json(&self) -> Result<Vec<u8>, HttpError> {
        self.dispatch(&RestRoute::current_user(), Outgoing::Empty, None, true)
            .await
    }

    pub async fn logout(&self) -> Result<(), HttpError> {
        let _ = self
            .dispatch(
                &RestRoute::logout(),
                Outgoing::json(&EmptyBody {})?,
                None,
                true,
            )
            .await;
        self.set_token(None).await;
        Ok(())
    }

    pub async fn send_channel_message(
        &self,
        channel_id: rusticord_core::Snowflake,
        payload_json: &str,
        files: &[UploadFile],
        on_progress: Option<Arc<dyn Fn(UploadProgress) + Send + Sync>>,
        cancel: Option<CancelFlag>,
        captcha: Option<&CaptchaSolution>,
    ) -> Result<Vec<u8>, HttpError> {
        let route = RestRoute::create_channel_message(channel_id);
        if files.is_empty() {
            let payload: serde_json::Value =
                serde_json::from_str(payload_json).map_err(|_| HttpError::InvalidJson)?;
            return self
                .dispatch(&route, Outgoing::json(&payload)?, captcha, true)
                .await;
        }
        if cancel.as_ref().is_some_and(CancelFlag::is_cancelled) {
            return Err(HttpError::Cancelled);
        }
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let boundary = make_boundary(unique);
        let body = encode_multipart(&boundary, payload_json, files);
        self.dispatch(
            &route,
            Outgoing::Multipart {
                content_type: multipart_content_type(&boundary),
                body,
                on_progress,
                cancel,
            },
            captcha,
            true,
        )
        .await
    }

    async fn dispatch(
        &self,
        route: &RestRoute,
        outgoing: Outgoing,
        captcha: Option<&CaptchaSolution>,
        authorize: bool,
    ) -> Result<Vec<u8>, HttpError> {
        let mut last_rate_limit = None;
        for attempt in 0..MAX_RETRY_ATTEMPTS {
            loop {
                let delay = {
                    let mut limiter = self.limiter.lock().await;
                    limiter.delay_before_send(route, Instant::now())
                };
                if delay.is_zero() {
                    let mut limiter = self.limiter.lock().await;
                    limiter.note_send(Instant::now());
                    break;
                }
                tokio::time::sleep(delay).await;
            }
            let token = if authorize {
                self.token.lock().await.clone()
            } else {
                None
            };
            let mut request = self
                .http
                .request(route.method.as_reqwest(), rest_url(route));
            request = apply_outgoing(request, &outgoing)?;
            if let Some(token) = token.as_ref() {
                request = request.header("authorization", token.as_str());
            }
            if let Some(captcha) = captcha {
                request = apply_captcha(request, captcha);
            }
            let response = match request.send().await {
                Ok(response) => response,
                Err(_) => {
                    if outgoing.cancelled() {
                        return Err(HttpError::Cancelled);
                    }
                    return Err(HttpError::Transport);
                }
            };
            let status = response.status().as_u16();
            let headers = header_map(response.headers());
            {
                let mut limiter = self.limiter.lock().await;
                limiter.observe_headers(route, &headers, Instant::now());
            }
            let bytes = response.bytes().await.map_err(|_| HttpError::Transport)?;
            if status == 429 {
                let retry_after = DiscordErrorBody::parse_json(&bytes)
                    .and_then(
                        |body| match body.into_http_error(status, headers.retry_after) {
                            HttpError::RateLimited(limited) => Some(limited.retry_after),
                            _ => headers.retry_after,
                        },
                    )
                    .or(headers.retry_after)
                    .unwrap_or(std::time::Duration::from_secs(1));
                {
                    let mut limiter = self.limiter.lock().await;
                    limiter.observe_rate_limited(route, retry_after, Instant::now());
                }
                let wait = retry_wait(retry_after, attempt, jitter_per_mille(attempt));
                last_rate_limit = Some(retry_after);
                tokio::time::sleep(wait).await;
                continue;
            }
            if (200..300).contains(&status) {
                return Ok(bytes.to_vec());
            }
            if let Some(body) = DiscordErrorBody::parse_json(&bytes) {
                return Err(body.into_http_error(status, headers.retry_after));
            }
            return Err(HttpError::Api(ApiError {
                status,
                code: ApiErrorCode::General,
                message: String::new(),
            }));
        }
        Err(HttpError::RateLimited(RateLimited {
            retry_after: last_rate_limit.unwrap_or(std::time::Duration::from_secs(1)),
            global: false,
            scope: crate::error::RateLimitScope::Unknown,
        }))
    }
}

impl Outgoing {
    fn json<T: Serialize>(body: &T) -> Result<Self, HttpError> {
        let payload = serde_json::to_vec(body).map_err(|_| HttpError::InvalidJson)?;
        Ok(Self::Json(payload))
    }

    fn cancelled(&self) -> bool {
        match self {
            Self::Multipart {
                cancel: Some(flag), ..
            } => flag.is_cancelled(),
            _ => false,
        }
    }
}

#[derive(Serialize)]
struct EmptyBody {}

fn apply_outgoing(
    request: reqwest::RequestBuilder,
    outgoing: &Outgoing,
) -> Result<reqwest::RequestBuilder, HttpError> {
    match outgoing {
        Outgoing::Empty => Ok(request),
        Outgoing::Json(payload) => Ok(request
            .header("content-type", "application/json")
            .body(payload.clone())),
        Outgoing::Multipart {
            content_type,
            body,
            on_progress,
            cancel,
        } => {
            if cancel.as_ref().is_some_and(CancelFlag::is_cancelled) {
                return Err(HttpError::Cancelled);
            }
            let length = body.len();
            let stream = ProgressBody::new(body.clone(), on_progress.clone(), cancel.clone());
            Ok(request
                .header("content-type", content_type.as_str())
                .header("content-length", length.to_string())
                .body(reqwest::Body::wrap_stream(stream)))
        }
    }
}

fn apply_captcha(
    request: reqwest::RequestBuilder,
    captcha: &CaptchaSolution,
) -> reqwest::RequestBuilder {
    let mut request = request.header("x-captcha-key", captcha.key.as_str());
    if let Some(session_id) = captcha.session_id.as_deref() {
        request = request.header("x-captcha-session-id", session_id);
    }
    if let Some(rqtoken) = captcha.rqtoken.as_deref() {
        request = request.header("x-captcha-rqtoken", rqtoken);
    }
    request
}

fn header_map(headers: &HeaderMap) -> RateLimitHeaders {
    RateLimitHeaders::parse(|name| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(String::from)
    })
}

fn jitter_per_mille(attempt: u8) -> u16 {
    let mixed = u16::from(attempt).wrapping_mul(37).wrapping_add(113);
    50 + (mixed % 200)
}
