mod auth;
mod client;
mod error;
mod rate_limit;
mod remote_auth;
mod route;
mod runtime;
mod upload;

pub use auth::{
    EncryptedTokenBody, LoginCredentials, LoginOutcome, LoginSettings, MfaMethod, MfaSuccessBody,
    Password,
};
pub use client::{CaptchaSolution, RestClient};
pub use error::{
    ApiError, ApiErrorCode, CaptchaChallenge, CaptchaService, HttpError, RateLimitScope,
    RateLimited,
};
pub use rate_limit::{RateLimitHeaders, RateLimiter, retry_wait};
pub use remote_auth::{
    RemoteAuthAction, RemoteAuthClientMessage, RemoteAuthError, RemoteAuthServerMessage,
    RemoteAuthSession, RemoteUserPreview, parse_user_payload, qr_login_url,
    remote_auth_gateway_url, remote_auth_origin,
};
pub use route::{BucketKey, HttpMethod, RestRoute, discord_api_origin, rest_root, rest_url};
pub use runtime::runtime_handle;
pub use upload::{
    CancelFlag, ProgressBody, UPLOAD_CHUNK_BYTES, UploadFile, UploadProgress, encode_multipart,
    make_boundary, multipart_content_type,
};
