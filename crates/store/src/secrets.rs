use keyring::Entry;
use rusticord_platform::APPLICATION_IDENTIFIER;
use zeroize::Zeroize;

use crate::StoreError;

const TOKEN_USER: &str = "discord-token";

pub struct SessionToken {
    value: String,
}

impl SessionToken {
    pub fn new(value: String) -> Self {
        Self { value }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl Drop for SessionToken {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}

impl std::fmt::Debug for SessionToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("[redacted]")
    }
}

pub struct TokenStore;

impl TokenStore {
    pub fn save(token: &str) -> Result<(), StoreError> {
        entry()?
            .set_password(token)
            .map_err(|_| StoreError::Secrets)
    }

    pub fn load() -> Result<Option<SessionToken>, StoreError> {
        match entry()?.get_password() {
            Ok(value) => Ok(Some(SessionToken::new(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(StoreError::Secrets),
        }
    }

    pub fn delete() -> Result<(), StoreError> {
        match entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(StoreError::Secrets),
        }
    }
}

fn entry() -> Result<Entry, StoreError> {
    Entry::new(APPLICATION_IDENTIFIER, TOKEN_USER).map_err(|_| StoreError::Secrets)
}

#[cfg(test)]
mod tests {
    use super::SessionToken;

    #[test]
    fn debug_hides_token_bytes() {
        let token = SessionToken::new(String::from("secret.token.value"));
        assert_eq!(format!("{token:?}"), "[redacted]");
        assert_eq!(token.as_str(), "secret.token.value");
    }
}
