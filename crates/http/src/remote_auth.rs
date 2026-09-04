use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use rand::rngs::OsRng;
use rsa::pkcs8::EncodePublicKey;
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use rusticord_core::Snowflake;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::route::discord_api_origin;

#[derive(Debug, thiserror::Error)]
pub enum RemoteAuthError {
    #[error("remote auth handshake failed")]
    Handshake,
    #[error("remote auth payload was not valid")]
    InvalidPayload,
    #[error("remote auth fingerprint mismatch")]
    FingerprintMismatch,
    #[error("remote auth cryptography failed")]
    Crypto,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum RemoteAuthServerMessage {
    Hello {
        timeout_ms: u64,
        heartbeat_interval: u64,
    },
    HeartbeatAck,
    NonceProof {
        encrypted_nonce: String,
    },
    PendingRemoteInit {
        fingerprint: String,
    },
    PendingTicket {
        encrypted_user_payload: String,
    },
    PendingLogin {
        ticket: String,
    },
    Cancel,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum RemoteAuthClientMessage {
    Init { encoded_public_key: String },
    Heartbeat,
    NonceProof { nonce: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteUserPreview {
    pub user_id: Snowflake,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub username: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RemoteAuthAction {
    Send(RemoteAuthClientMessage),
    StartHeartbeat { interval_ms: u64, timeout_ms: u64 },
    ShowQr { fingerprint: String, url: String },
    ShowUser(RemoteUserPreview),
    Complete { ticket: String },
    Cancelled,
}

pub(crate) enum Phase {
    WaitHello,
    WaitNonce,
    WaitFingerprint,
    WaitTicket,
    WaitLogin,
    Closed,
}

pub struct RemoteAuthSession {
    pub(crate) key: RsaPrivateKey,
    encoded_public_key: String,
    fingerprint: String,
    pub(crate) phase: Phase,
}

impl RemoteAuthSession {
    pub fn generate() -> Result<Self, RemoteAuthError> {
        let key = RsaPrivateKey::new(&mut OsRng, 2048).map_err(|_| RemoteAuthError::Crypto)?;
        Self::from_key(key)
    }

    fn from_key(key: RsaPrivateKey) -> Result<Self, RemoteAuthError> {
        let public = RsaPublicKey::from(&key);
        let der = public
            .to_public_key_der()
            .map_err(|_| RemoteAuthError::Crypto)?;
        let encoded_public_key = STANDARD.encode(der.as_bytes());
        let fingerprint = fingerprint_from_spki(der.as_bytes());
        Ok(Self {
            key,
            encoded_public_key,
            fingerprint,
            phase: Phase::WaitHello,
        })
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn encoded_public_key(&self) -> &str {
        &self.encoded_public_key
    }

    pub fn decrypt_oaep(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RemoteAuthError> {
        self.key
            .decrypt(Oaep::new::<Sha256>(), ciphertext)
            .map_err(|_| RemoteAuthError::Crypto)
    }

    pub fn decrypt_token(&self, encrypted_token: &str) -> Result<String, RemoteAuthError> {
        let ciphertext = STANDARD
            .decode(encrypted_token.as_bytes())
            .map_err(|_| RemoteAuthError::InvalidPayload)?;
        let plain = self.decrypt_oaep(&ciphertext)?;
        String::from_utf8(plain).map_err(|_| RemoteAuthError::InvalidPayload)
    }

    pub fn on_json(&mut self, json: &str) -> Result<Vec<RemoteAuthAction>, RemoteAuthError> {
        let message: RemoteAuthServerMessage =
            serde_json::from_str(json).map_err(|_| RemoteAuthError::InvalidPayload)?;
        self.on_message(message)
    }

    pub fn on_message(
        &mut self,
        message: RemoteAuthServerMessage,
    ) -> Result<Vec<RemoteAuthAction>, RemoteAuthError> {
        match (&self.phase, message) {
            (
                Phase::WaitHello,
                RemoteAuthServerMessage::Hello {
                    timeout_ms,
                    heartbeat_interval,
                },
            ) => {
                self.phase = Phase::WaitNonce;
                Ok(vec![
                    RemoteAuthAction::StartHeartbeat {
                        interval_ms: heartbeat_interval,
                        timeout_ms,
                    },
                    RemoteAuthAction::Send(RemoteAuthClientMessage::Init {
                        encoded_public_key: self.encoded_public_key.clone(),
                    }),
                ])
            }
            (Phase::WaitNonce, RemoteAuthServerMessage::NonceProof { encrypted_nonce }) => {
                let ciphertext = STANDARD
                    .decode(encrypted_nonce.as_bytes())
                    .map_err(|_| RemoteAuthError::InvalidPayload)?;
                let nonce = self.decrypt_oaep(&ciphertext)?;
                self.phase = Phase::WaitFingerprint;
                Ok(vec![RemoteAuthAction::Send(
                    RemoteAuthClientMessage::NonceProof {
                        nonce: URL_SAFE_NO_PAD.encode(nonce),
                    },
                )])
            }
            (
                Phase::WaitFingerprint,
                RemoteAuthServerMessage::PendingRemoteInit { fingerprint },
            ) => {
                if fingerprint != self.fingerprint {
                    self.phase = Phase::Closed;
                    return Err(RemoteAuthError::FingerprintMismatch);
                }
                self.phase = Phase::WaitTicket;
                Ok(vec![RemoteAuthAction::ShowQr {
                    fingerprint: fingerprint.clone(),
                    url: qr_login_url(&fingerprint),
                }])
            }
            (
                Phase::WaitTicket,
                RemoteAuthServerMessage::PendingTicket {
                    encrypted_user_payload,
                },
            ) => {
                let ciphertext = STANDARD
                    .decode(encrypted_user_payload.as_bytes())
                    .map_err(|_| RemoteAuthError::InvalidPayload)?;
                let plain = self.decrypt_oaep(&ciphertext)?;
                self.phase = Phase::WaitLogin;
                Ok(vec![RemoteAuthAction::ShowUser(parse_user_payload(
                    &plain,
                )?)])
            }
            (Phase::WaitLogin, RemoteAuthServerMessage::PendingLogin { ticket }) => {
                self.phase = Phase::Closed;
                Ok(vec![RemoteAuthAction::Complete { ticket }])
            }
            (Phase::WaitTicket | Phase::WaitLogin, RemoteAuthServerMessage::Cancel) => {
                self.phase = Phase::Closed;
                Ok(vec![RemoteAuthAction::Cancelled])
            }
            (_, RemoteAuthServerMessage::HeartbeatAck) => Ok(Vec::new()),
            _ => Err(RemoteAuthError::Handshake),
        }
    }
}

pub fn qr_login_url(fingerprint: &str) -> String {
    let mut url = discord_api_origin();
    url.push_str("/ra/");
    url.push_str(fingerprint);
    url
}

pub fn remote_auth_gateway_url() -> String {
    let mut url = String::from("wss:");
    url.push('/');
    url.push('/');
    url.push_str("remote-auth-gateway.discord.gg/?v=2");
    url
}

pub fn remote_auth_origin() -> String {
    discord_api_origin()
}

fn fingerprint_from_spki(spki: &[u8]) -> String {
    let digest = Sha256::digest(spki);
    URL_SAFE_NO_PAD.encode(digest)
}

pub fn parse_user_payload(bytes: &[u8]) -> Result<RemoteUserPreview, RemoteAuthError> {
    let text = std::str::from_utf8(bytes).map_err(|_| RemoteAuthError::InvalidPayload)?;
    let mut parts = text.splitn(4, ':');
    let user_id = parts
        .next()
        .ok_or(RemoteAuthError::InvalidPayload)?
        .parse::<Snowflake>()
        .map_err(|_| RemoteAuthError::InvalidPayload)?;
    let discriminator = String::from(parts.next().ok_or(RemoteAuthError::InvalidPayload)?);
    let avatar_raw = parts.next().ok_or(RemoteAuthError::InvalidPayload)?;
    let username = String::from(parts.next().ok_or(RemoteAuthError::InvalidPayload)?);
    let avatar = if avatar_raw == "0" || avatar_raw.is_empty() {
        None
    } else {
        Some(String::from(avatar_raw))
    };
    Ok(RemoteUserPreview {
        user_id,
        discriminator,
        avatar,
        username,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        RemoteAuthAction, RemoteAuthClientMessage, RemoteAuthError, RemoteAuthServerMessage,
        RemoteAuthSession, parse_user_payload, qr_login_url, remote_auth_gateway_url,
    };
    use base64::Engine;
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    use rand::rngs::OsRng;
    use rsa::pkcs8::EncodePublicKey;
    use rsa::{Oaep, RsaPublicKey};
    use rusticord_core::Snowflake;
    use sha2::{Digest, Sha256};

    #[test]
    fn parses_hello_and_emits_init() {
        let mut session = RemoteAuthSession::generate().unwrap();
        let actions = session
            .on_json(r#"{"op":"hello","timeout_ms":120000,"heartbeat_interval":40000}"#)
            .unwrap();
        assert!(matches!(
            actions.as_slice(),
            [
                RemoteAuthAction::StartHeartbeat {
                    interval_ms: 40000,
                    timeout_ms: 120000
                },
                RemoteAuthAction::Send(RemoteAuthClientMessage::Init { .. })
            ]
        ));
    }

    #[test]
    fn nonce_proof_is_urlsafe_decrypted_bytes() {
        let mut session = RemoteAuthSession::generate().unwrap();
        let _ = session
            .on_message(RemoteAuthServerMessage::Hello {
                timeout_ms: 1,
                heartbeat_interval: 1,
            })
            .unwrap();
        let nonce = b"proof-bytes-for-remote-auth";
        let public = RsaPublicKey::from(&session.key);
        let ciphertext = public
            .encrypt(&mut OsRng, Oaep::new::<Sha256>(), nonce)
            .unwrap();
        let actions = session
            .on_message(RemoteAuthServerMessage::NonceProof {
                encrypted_nonce: STANDARD.encode(ciphertext),
            })
            .unwrap();
        let RemoteAuthAction::Send(RemoteAuthClientMessage::NonceProof { nonce: encoded }) =
            actions.into_iter().next().unwrap()
        else {
            let matched = false;
            assert!(matched);
            return;
        };
        assert_eq!(encoded, URL_SAFE_NO_PAD.encode(nonce));
    }

    #[test]
    fn fingerprint_must_match_public_key_digest() {
        let mut session = RemoteAuthSession::generate().unwrap();
        let _ = session
            .on_message(RemoteAuthServerMessage::Hello {
                timeout_ms: 1,
                heartbeat_interval: 1,
            })
            .unwrap();
        session.phase = super::Phase::WaitFingerprint;
        let public = RsaPublicKey::from(&session.key);
        let der = public.to_public_key_der().unwrap();
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(der.as_bytes()));
        assert_eq!(session.fingerprint(), expected);
        let err = session
            .on_message(RemoteAuthServerMessage::PendingRemoteInit {
                fingerprint: String::from("not-the-fingerprint"),
            })
            .unwrap_err();
        assert!(matches!(err, RemoteAuthError::FingerprintMismatch));
    }

    #[test]
    fn pending_init_builds_qr_url() {
        let mut session = RemoteAuthSession::generate().unwrap();
        session.phase = super::Phase::WaitFingerprint;
        let fingerprint = String::from(session.fingerprint());
        let actions = session
            .on_message(RemoteAuthServerMessage::PendingRemoteInit {
                fingerprint: fingerprint.clone(),
            })
            .unwrap();
        let expected = qr_login_url(&fingerprint);
        assert!(matches!(
            actions.as_slice(),
            [RemoteAuthAction::ShowQr { url, .. }] if *url == expected
        ));
        assert!(expected.starts_with("https:"));
        assert!(expected.contains("/ra/"));
    }

    #[test]
    fn user_payload_splits_on_first_three_colons() {
        let preview = parse_user_payload(b"852892297661906993:0:0:dolfies").unwrap();
        assert_eq!(preview.user_id, Snowflake::from_raw(852892297661906993));
        assert_eq!(preview.discriminator, "0");
        assert_eq!(preview.avatar, None);
        assert_eq!(preview.username, "dolfies");
    }

    #[test]
    fn gateway_url_uses_version_two() {
        let url = remote_auth_gateway_url();
        assert!(url.starts_with("wss:"));
        assert!(url.contains("remote-auth-gateway.discord.gg"));
        assert!(url.ends_with("v=2"));
    }
}
