use anyhow::Result;
use js_sys::Function;
use thiserror::Error;
use web_sys::WebSocket;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("cannot create websocket with url `{0}` due to `{1}`")]
    CreatingWebsocket(String, String),
    #[error("sending message failed due to `{0}`")]
    SendingMessage(String),
    #[allow(dead_code)]
    #[error("cannot add event listener with callback due to `{0}`")]
    AddingEventListener(String),
}
pub struct Transport {
    websocket: WebSocket,
}

impl Transport {
    /// Creates a new `Transport`.
    pub fn new(url: String) -> Result<Self> {
        let websocket = WebSocket::new(&url).map_err(|e| {
            TransportError::CreatingWebsocket(url, e.as_string().unwrap_or("unknown error".into()))
        })?;
        Ok(Self { websocket })
    }

    /// Sets closure to execute when a message is received on the websocket.
    pub fn set_onmessage(&self, function: &Function) {
        self.websocket.set_onmessage(Some(function));
    }

    /// Sets closure to execute when the websocket is open.
    pub fn set_onopen(&self, function: &Function) {
        self.websocket.set_onopen(Some(function));
    }

    /// Sets closure to execute when the websocket errors out.
    #[allow(dead_code)]
    pub fn set_onerror(&self, function: &Function) {
        self.websocket.set_onerror(Some(function));
    }

    /// Sets closure to execute when the websocket is closed.
    #[allow(dead_code)]
    pub fn set_onclose(&self, function: &Function) {
        self.websocket.set_onclose(Some(function));
    }

    /// Sends a new message to the websocket.
    #[allow(dead_code)]
    pub fn send(&self, message: &str) -> Result<()> {
        self.websocket.send_with_str(message).map_err(|e| {
            TransportError::SendingMessage(e.as_string().unwrap_or("unknown error".into()))
        })?;
        Ok(())
    }

    /// Returns raw websocket object.
    pub fn get_raw(&self) -> WebSocket {
        self.websocket.clone()
    }

    /// Adds a new event listener with callback.
    #[allow(dead_code)]
    pub fn add_event_listener_with_callback(&self, event: &str, callback: &Function) -> Result<()> {
        self.websocket
            .add_event_listener_with_callback(event, callback)
            .map_err(|e| {
                TransportError::AddingEventListener(e.as_string().unwrap_or("unknown error".into()))
            })?;
        Ok(())
    }
}
