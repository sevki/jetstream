use crate::DynamicProtocol;
use futures::StreamExt;
use jetstream_rpc::context::{Context, Contextual, RemoteAddr};
use std::{net::IpAddr, str::FromStr};
use worker::{Error, WebSocket};

unsafe impl Send for WebSocketTransport {}
unsafe impl Sync for WebSocketTransport {}

pub struct WebSocketTransport {
    ip: String,
    websocket: WebSocket,
}

impl WebSocketTransport {
    pub fn new(websocket: WebSocket, ip: String) -> Result<Self, Error> {
        websocket.accept()?;
        Ok(Self { ip, websocket })
    }
}

impl Contextual for WebSocketTransport {
    fn context(&self) -> Context {
        let ip = IpAddr::from_str(&self.ip).ok();
        Context::new(Some(RemoteAddr::IpAddr(ip.unwrap())), None)
    }
}

impl WebSocketTransport {
    pub async fn handle(&mut self, handler: &mut Box<dyn DynamicProtocol>) {
        worker::console_log!("WebSocketTransport::handle started");
        let mut events = self.websocket.events().unwrap();
        worker::console_log!("Got event stream, waiting for events...");

        while let Some(event) = events.next().await {
            worker::console_log!("Received event: {:?}", event);
            match event {
                Ok(worker::WebsocketEvent::Message(message_event)) => {
                    worker::console_log!("Processing message event");
                    let bytes = message_event.bytes().unwrap();
                    worker::console_log!("Message size: {} bytes", bytes.len());

                    let data = handler
                        .rpc(self.context(), bytes.as_slice())
                        .await
                        .unwrap();

                    worker::console_log!(
                        "RPC completed, sending response: {} bytes",
                        data.len()
                    );
                    self.websocket.send_with_bytes(data).unwrap();
                    worker::console_log!("Response sent");
                }
                Ok(worker::WebsocketEvent::Close(_close_event)) => {
                    worker::console_log!("WebSocket close event received");
                    break;
                }
                Err(e) => {
                    worker::console_log!("WebSocket error: {:?}", e);
                    break;
                }
            }
        }
        worker::console_log!("WebSocketTransport::handle finished");
    }
}
