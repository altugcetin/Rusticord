use rusticord_core::Snowflake;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub fn as_reqwest(self) -> reqwest::Method {
        match self {
            Self::Get => reqwest::Method::GET,
            Self::Post => reqwest::Method::POST,
            Self::Put => reqwest::Method::PUT,
            Self::Patch => reqwest::Method::PATCH,
            Self::Delete => reqwest::Method::DELETE,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RestRoute {
    pub method: HttpMethod,
    pub path: String,
    pub major: Option<Snowflake>,
}

impl RestRoute {
    pub fn login() -> Self {
        Self {
            method: HttpMethod::Post,
            path: String::from("/auth/login"),
            major: None,
        }
    }

    pub fn mfa_totp() -> Self {
        Self {
            method: HttpMethod::Post,
            path: String::from("/auth/mfa/totp"),
            major: None,
        }
    }

    pub fn mfa_sms() -> Self {
        Self {
            method: HttpMethod::Post,
            path: String::from("/auth/mfa/sms"),
            major: None,
        }
    }

    pub fn mfa_sms_send() -> Self {
        Self {
            method: HttpMethod::Post,
            path: String::from("/auth/mfa/sms/send"),
            major: None,
        }
    }

    pub fn mfa_backup() -> Self {
        Self {
            method: HttpMethod::Post,
            path: String::from("/auth/mfa/backup"),
            major: None,
        }
    }

    pub fn mfa_webauthn() -> Self {
        Self {
            method: HttpMethod::Post,
            path: String::from("/auth/mfa/webauthn"),
            major: None,
        }
    }

    pub fn logout() -> Self {
        Self {
            method: HttpMethod::Post,
            path: String::from("/auth/logout"),
            major: None,
        }
    }

    pub fn channel_messages(channel_id: Snowflake) -> Self {
        Self {
            method: HttpMethod::Get,
            path: format!("/channels/{channel_id}/messages"),
            major: Some(channel_id),
        }
    }

    pub fn create_channel_message(channel_id: Snowflake) -> Self {
        Self {
            method: HttpMethod::Post,
            path: format!("/channels/{channel_id}/messages"),
            major: Some(channel_id),
        }
    }

    pub fn guild_channels(guild_id: Snowflake) -> Self {
        Self {
            method: HttpMethod::Get,
            path: format!("/guilds/{guild_id}/channels"),
            major: Some(guild_id),
        }
    }

    pub fn current_user() -> Self {
        Self {
            method: HttpMethod::Get,
            path: String::from("/users/@me"),
            major: None,
        }
    }

    pub fn remote_auth_login() -> Self {
        Self {
            method: HttpMethod::Post,
            path: String::from("/users/@me/remote-auth/login"),
            major: None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BucketKey {
    pub hash: String,
    pub major: Option<Snowflake>,
}

impl BucketKey {
    pub fn from_route(route: &RestRoute) -> Self {
        Self {
            hash: format!("{:?}:{}", route.method, route.path),
            major: route.major,
        }
    }

    pub fn from_header(hash: &str, major: Option<Snowflake>) -> Self {
        Self {
            hash: String::from(hash),
            major,
        }
    }
}

pub fn discord_api_origin() -> String {
    let mut origin = String::from("https:");
    origin.push('/');
    origin.push('/');
    origin.push_str("discord.com");
    origin
}

pub fn rest_root() -> String {
    let mut root = discord_api_origin();
    root.push_str("/api/v10");
    root
}

pub fn rest_url(route: &RestRoute) -> String {
    let mut url = rest_root();
    url.push_str(&route.path);
    url
}

#[cfg(test)]
mod tests {
    use super::{HttpMethod, RestRoute, discord_api_origin, rest_url};
    use rusticord_core::Snowflake;

    #[test]
    fn origin_uses_https_without_comment_tokens_in_literals() {
        let origin = discord_api_origin();
        assert!(origin.starts_with("https:"));
        assert!(origin.contains("discord.com"));
    }

    #[test]
    fn channel_route_carries_channel_as_major() {
        let channel = Snowflake::from_raw(1);
        let route = RestRoute::channel_messages(channel);
        assert_eq!(route.method, HttpMethod::Get);
        assert_eq!(route.major, Some(channel));
        assert!(rest_url(&route).ends_with("/channels/1/messages"));
    }
}
