use rusticord_core::Snowflake;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Clone)]
pub struct Password {
    value: String,
}

impl Password {
    pub fn new(value: String) -> Self {
        Self { value }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl Drop for Password {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}

impl std::fmt::Debug for Password {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("[redacted]")
    }
}

#[derive(Clone, Debug)]
pub struct LoginCredentials {
    pub login: String,
    pub password: Password,
    pub undelete: bool,
}

#[derive(Debug, Serialize)]
pub struct LoginBody<'a> {
    pub login: &'a str,
    pub password: &'a str,
    pub undelete: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoginSettings {
    pub locale: Option<String>,
    pub theme: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoginSuccessBody {
    pub user_id: Snowflake,
    pub token: Option<String>,
    pub ticket: Option<String>,
    pub login_instance_id: Option<String>,
    pub mfa: Option<bool>,
    pub totp: Option<bool>,
    pub sms: Option<bool>,
    pub backup: Option<bool>,
    pub webauthn: Option<String>,
    pub user_settings: Option<LoginSettings>,
    pub required_actions: Option<Vec<String>>,
    pub suspended_user_token: Option<String>,
}

#[derive(Clone, Debug)]
pub enum LoginOutcome {
    Completed {
        user_id: Snowflake,
        token: String,
        settings: Option<LoginSettings>,
        required_actions: Vec<String>,
    },
    MfaRequired {
        user_id: Snowflake,
        ticket: String,
        login_instance_id: Option<String>,
        totp: bool,
        sms: bool,
        backup: bool,
        webauthn: Option<String>,
    },
    Suspended {
        user_id: Snowflake,
        token: String,
    },
}

impl LoginSuccessBody {
    pub fn into_outcome(self) -> Option<LoginOutcome> {
        if let Some(token) = self.suspended_user_token {
            return Some(LoginOutcome::Suspended {
                user_id: self.user_id,
                token,
            });
        }
        if self.mfa.unwrap_or(false) {
            let ticket = self.ticket?;
            return Some(LoginOutcome::MfaRequired {
                user_id: self.user_id,
                ticket,
                login_instance_id: self.login_instance_id,
                totp: self.totp.unwrap_or(false),
                sms: self.sms.unwrap_or(false),
                backup: self.backup.unwrap_or(false),
                webauthn: self.webauthn,
            });
        }
        let token = self.token?;
        Some(LoginOutcome::Completed {
            user_id: self.user_id,
            token,
            settings: self.user_settings,
            required_actions: self.required_actions.unwrap_or_default(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MfaMethod {
    Totp,
    Sms,
    Backup,
    Webauthn,
}

impl MfaMethod {
    pub fn route(self) -> crate::route::RestRoute {
        match self {
            Self::Totp => crate::route::RestRoute::mfa_totp(),
            Self::Sms => crate::route::RestRoute::mfa_sms(),
            Self::Backup => crate::route::RestRoute::mfa_backup(),
            Self::Webauthn => crate::route::RestRoute::mfa_webauthn(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MfaBody<'a> {
    pub ticket: &'a str,
    pub code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_instance_id: Option<&'a str>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MfaSuccessBody {
    pub token: String,
    pub user_settings: Option<LoginSettings>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MfaSmsSendBody<'a> {
    pub ticket: &'a str,
}

#[derive(Debug, Serialize)]
pub struct RemoteAuthLoginBody<'a> {
    pub ticket: &'a str,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EncryptedTokenBody {
    pub encrypted_token: String,
}

#[cfg(test)]
mod tests {
    use super::LoginSuccessBody;

    #[test]
    fn completed_login_requires_token() {
        let json = r#"{"user_id":"852892297661906993","token":"secret.token","user_settings":{"locale":"tr","theme":"dark"}}"#;
        let body: LoginSuccessBody = serde_json::from_str(json).unwrap();
        assert!(matches!(
            body.into_outcome(),
            Some(super::LoginOutcome::Completed { .. })
        ));
    }

    #[test]
    fn mfa_login_requires_ticket() {
        let json = r#"{"user_id":"852892297661906993","mfa":true,"sms":true,"ticket":"ticket.value","totp":true,"backup":true}"#;
        let body: LoginSuccessBody = serde_json::from_str(json).unwrap();
        let super::LoginOutcome::MfaRequired {
            totp, sms, backup, ..
        } = body.into_outcome().unwrap()
        else {
            let matched = false;
            assert!(matched);
            return;
        };
        assert!(totp && sms && backup);
    }
}
