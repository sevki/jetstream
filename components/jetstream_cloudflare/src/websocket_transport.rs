use crate::{DynamicProtocol, Shutdown};
use futures::StreamExt;
use jetstream_rpc::context::{Context, Contextual, RemoteAddr};
use std::{net::IpAddr, str::FromStr};
use worker::{Error, WebSocket};

unsafe impl Send for WebSocketTransport {}
unsafe impl Sync for WebSocketTransport {}

pub struct WebSocketTransport {
    ip: String,
    websocket: WebSocket,
    shutdown_reason: Option<Shutdown>,
}

impl WebSocketTransport {
    pub fn new(websocket: WebSocket, ip: String) -> Result<Self, Error> {
        websocket.accept()?;
        Ok(Self {
            ip,
            websocket,
            shutdown_reason: None,
        })
    }
}

impl Contextual for WebSocketTransport {
    fn context(&self) -> Context {
        let ip = IpAddr::from_str(&self.ip).ok().map(RemoteAddr::IpAddr);
        Context::new(ip, None)
    }
}

impl WebSocketTransport {
    pub fn shutdown(&mut self, sd: Shutdown) -> worker::Result<()> {
        sd.shutdown(&mut self.websocket)
    }
}

impl WebSocketTransport {
    pub async fn handle(&mut self, handler: &mut Box<dyn DynamicProtocol>) {
        let mut events = self.websocket.events().unwrap();

        while let Some(event) = events.next().await {
            match event {
                Ok(worker::WebsocketEvent::Message(message_event)) => {
                    let bytes = message_event.bytes().unwrap();

                    let Ok(data) =
                        handler.rpc(self.context(), bytes.as_slice()).await
                    else {
                        self.shutdown_reason = Some(Shutdown::RpcFailure);
                        return;
                    };

                    self.websocket.send_with_bytes(data).unwrap();
                }
                Ok(worker::WebsocketEvent::Close(_close_event)) => {
                    break;
                }
                Err(e) => {
                    worker::console_log!("WebSocket error: {:?}", e);
                    break;
                }
            }
        }
    }
}

impl Drop for WebSocketTransport {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown_reason.take() {
            shutdown.shutdown(&mut self.websocket).unwrap();
        } else {
            _ = self.websocket.close::<String>(None, None);
        }
    }
}
