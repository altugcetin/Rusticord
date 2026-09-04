use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rustls::pki_types::ServerName;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderValue, Request};

use crate::remote_auth::{
    RemoteAuthAction, RemoteAuthClientMessage, RemoteAuthError, RemoteAuthSession,
    remote_auth_gateway_url, remote_auth_origin,
};

const GATEWAY_HOST: &str = "remote-auth-gateway.discord.gg";

pub fn remote_auth_host() -> &'static str {
    GATEWAY_HOST
}

pub fn rustls_client_config() -> Result<rustls::ClientConfig, RemoteAuthError> {
    let _ = rustls_graviola::default_provider().install_default();
    let provider = Arc::new(rustls_graviola::default_provider());
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|_| RemoteAuthError::Handshake)
        .map(|builder| builder.with_root_certificates(roots).with_no_client_auth())
}

pub fn remote_auth_http_request() -> Result<Request<()>, RemoteAuthError> {
    let url = remote_auth_gateway_url();
    let mut request = url
        .into_client_request()
        .map_err(|_| RemoteAuthError::Handshake)?;
    let origin = remote_auth_origin();
    let origin = HeaderValue::from_str(&origin).map_err(|_| RemoteAuthError::Handshake)?;
    request.headers_mut().insert("origin", origin);
    Ok(request)
}

pub async fn complete_remote_auth<F>(
    mut session: RemoteAuthSession,
    mut on_action: F,
) -> Result<String, RemoteAuthError>
where
    F: FnMut(RemoteAuthAction),
{
    let request = remote_auth_http_request()?;
    let config = rustls_client_config()?;
    let server_name = ServerName::try_from(GATEWAY_HOST).map_err(|_| RemoteAuthError::Handshake)?;
    let tcp = TcpStream::connect((GATEWAY_HOST, 443))
        .await
        .map_err(|_| RemoteAuthError::Handshake)?;
    let tls = TlsConnector::from(Arc::new(config))
        .connect(server_name, tcp)
        .await
        .map_err(|_| RemoteAuthError::Handshake)?;
    let (stream, _) = tokio_tungstenite::client_async(request, tls)
        .await
        .map_err(|_| RemoteAuthError::Handshake)?;
    let (mut sink, mut incoming) = stream.split();
    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_secs(3600),
        Duration::from_secs(3600),
    );
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                send_client(&mut sink, &RemoteAuthClientMessage::Heartbeat).await?;
            }
            frame = incoming.next() => {
                let Some(frame) = frame else {
                    return Err(RemoteAuthError::Handshake);
                };
                let frame = frame.map_err(|_| RemoteAuthError::Handshake)?;
                match frame {
                    Message::Text(text) => {
                        let actions = session.on_json(text.as_str())?;
                        for action in actions {
                            let mut finished = None;
                            match &action {
                                RemoteAuthAction::Send(message) => {
                                    send_client(&mut sink, message).await?;
                                }
                                RemoteAuthAction::StartHeartbeat { interval_ms, .. } => {
                                    let period = Duration::from_millis((*interval_ms).max(1));
                                    heartbeat = tokio::time::interval_at(
                                        tokio::time::Instant::now() + period,
                                        period,
                                    );
                                    heartbeat.set_missed_tick_behavior(
                                        tokio::time::MissedTickBehavior::Delay,
                                    );
                                }
                                RemoteAuthAction::Complete { ticket } => {
                                    finished = Some(ticket.clone());
                                }
                                RemoteAuthAction::Cancelled => {
                                    on_action(action);
                                    return Err(RemoteAuthError::Handshake);
                                }
                                RemoteAuthAction::ShowQr { .. }
                                | RemoteAuthAction::ShowUser(_) => {}
                            }
                            on_action(action);
                            if let Some(ticket) = finished {
                                return Ok(ticket);
                            }
                        }
                    }
                    Message::Ping(payload) => {
                        sink.send(Message::Pong(payload))
                            .await
                            .map_err(|_| RemoteAuthError::Handshake)?;
                    }
                    Message::Close(_) => return Err(RemoteAuthError::Handshake),
                    _ => {}
                }
            }
        }
    }
}

async fn send_client<S>(
    sink: &mut S,
    message: &RemoteAuthClientMessage,
) -> Result<(), RemoteAuthError>
where
    S: SinkExt<Message> + Unpin,
{
    let json = serde_json::to_string(message).map_err(|_| RemoteAuthError::InvalidPayload)?;
    sink.send(Message::Text(json.into()))
        .await
        .map_err(|_| RemoteAuthError::Handshake)
}

#[cfg(test)]
mod tests {
    use super::{remote_auth_host, remote_auth_http_request, rustls_client_config};
    use crate::remote_auth::remote_auth_origin;

    #[test]
    fn request_carries_discord_origin() {
        let request = remote_auth_http_request().unwrap();
        let origin = request.headers().get("origin").unwrap().to_str().unwrap();
        assert_eq!(origin, remote_auth_origin());
        assert!(origin.starts_with("https:"));
        assert_eq!(remote_auth_host(), "remote-auth-gateway.discord.gg");
    }

    #[test]
    fn tls_config_installs_without_ring() {
        assert!(rustls_client_config().is_ok());
    }
}
