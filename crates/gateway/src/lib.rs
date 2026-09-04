mod zlib;
mod zstd;

pub use zlib::ZlibStreamDecoder;
pub use zstd::ZstdStreamDecoder;

#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("gateway payload was not valid")]
    InvalidPayload,
    #[error("gateway compression failed")]
    Compression,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Opcode {
    Dispatch,
    Heartbeat,
    Identify,
    PresenceUpdate,
    VoiceStateUpdate,
    Resume,
    Reconnect,
    RequestGuildMembers,
    InvalidSession,
    Hello,
    HeartbeatAck,
    Unknown(u8),
}

impl Opcode {
    pub fn from_raw(code: u8) -> Self {
        match code {
            0 => Self::Dispatch,
            1 => Self::Heartbeat,
            2 => Self::Identify,
            3 => Self::PresenceUpdate,
            4 => Self::VoiceStateUpdate,
            6 => Self::Resume,
            7 => Self::Reconnect,
            8 => Self::RequestGuildMembers,
            9 => Self::InvalidSession,
            10 => Self::Hello,
            11 => Self::HeartbeatAck,
            other => Self::Unknown(other),
        }
    }

    pub fn to_raw(self) -> u8 {
        match self {
            Self::Dispatch => 0,
            Self::Heartbeat => 1,
            Self::Identify => 2,
            Self::PresenceUpdate => 3,
            Self::VoiceStateUpdate => 4,
            Self::Resume => 6,
            Self::Reconnect => 7,
            Self::RequestGuildMembers => 8,
            Self::InvalidSession => 9,
            Self::Hello => 10,
            Self::HeartbeatAck => 11,
            Self::Unknown(code) => code,
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GatewayEnvelope {
    pub op: u8,
    #[serde(default)]
    pub d: serde_json::Value,
    #[serde(default)]
    pub s: Option<u64>,
    #[serde(default)]
    pub t: Option<String>,
}

impl GatewayEnvelope {
    pub fn opcode(&self) -> Opcode {
        Opcode::from_raw(self.op)
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Hello {
    pub heartbeat_interval: u64,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct IdentifyProperties {
    pub os: String,
    pub browser: String,
    pub device: String,
}

impl IdentifyProperties {
    pub fn rusticord() -> Self {
        Self {
            os: String::from(std::env::consts::OS),
            browser: String::from("Rusticord"),
            device: String::from("Rusticord"),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Identify {
    pub token: String,
    pub properties: IdentifyProperties,
    pub compress: bool,
    pub large_threshold: u16,
}

impl Identify {
    pub fn new(token: String) -> Self {
        Self {
            token,
            properties: IdentifyProperties::rusticord(),
            compress: false,
            large_threshold: 50,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Resume {
    pub token: String,
    pub session_id: String,
    pub seq: u64,
}

#[cfg(test)]
mod tests {
    use super::{GatewayEnvelope, Identify, Opcode};

    #[test]
    fn hello_opcode_roundtrips() {
        assert_eq!(Opcode::from_raw(10), Opcode::Hello);
        assert_eq!(Opcode::Hello.to_raw(), 10);
    }

    #[test]
    fn identify_uses_honest_client_name() {
        let identify = Identify::new(String::from("token"));
        assert_eq!(identify.properties.browser, "Rusticord");
        assert_eq!(identify.properties.device, "Rusticord");
        assert!(!identify.compress);
    }

    #[test]
    fn envelope_reads_sequence() {
        let json = r#"{"op":0,"d":{},"s":12,"t":"READY"}"#;
        let envelope: GatewayEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.opcode(), Opcode::Dispatch);
        assert_eq!(envelope.s, Some(12));
        assert_eq!(envelope.t.as_deref(), Some("READY"));
    }
}
