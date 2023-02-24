use crate::utils::timeout::enforce_timeout;

use super::transport::Transport;
use anyhow::Result;
use futures::{
    channel::{mpsc, oneshot},
    future, Sink, SinkExt, Stream, StreamExt,
};
use js_sys::JsString;
use json_rpc_types::{str_buf::StrBuf, Id, Request, Response, Version};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use thiserror::Error;
use tokio::sync::broadcast;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{Event, MessageEvent};

type PendingMessagesStore = Arc<Mutex<HashMap<u64, oneshot::Sender<Response<Value, Value>>>>>;

#[derive(Debug, Error)]
pub enum JsonRpcError {
    #[error("notification `${0}` was received without params")]
    NotificationWithoutParams(String),
}

pub struct JsonRpc {
    transport: Transport,
    message_id: AtomicU64,
    pending_messages: PendingMessagesStore,
    notification_tx: broadcast::Sender<Request<Value>>,
    timeout: Duration,
}

impl JsonRpc {
    /// Creates a new `JsonRpc`.
    pub fn new(url: String, timeout: Option<Duration>) -> Result<Self> {
        let transport = Transport::new(url)?;
        let pending_messages: PendingMessagesStore = Arc::new(Mutex::new(HashMap::new()));
        let timeout = timeout.unwrap_or(Duration::from_secs(30));

        // Register channel to receive notifications
        let (notification_tx, _) = broadcast::channel::<Request<Value>>(32);

        // Set onmessage callback to handle all received messages
        let pending_messages_c = pending_messages.clone();
        let notification_tx_c = notification_tx.clone();
        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |message: MessageEvent| {
            // Check response is string otherwise return
            let Ok(message) = message.data().dyn_into::<JsString>() else { return };
            let message = String::from(message);

            // Handle response message
            if let Ok(message) = serde_json::from_str::<Response<Value, Value>>(&message) {
                // Validate message
                let Some(Id::Num(res_id)) = message.id else { return };
                log::debug!("Response received: {:?}", message);

                // Return response to client, if any
                let Some(tx) = pending_messages_c.lock().unwrap().remove(&res_id) else { return };
                tx.send(message).unwrap(); //FIXME
                return;
            }

            // Handle notification message
            if let Ok(message) = serde_json::from_str::<Request<Value>>(&message) {
                if !message.is_notification() {
                    return;
                }
                let _ = notification_tx_c.send(message); // Ignores no receiver error
                return;
            }
        });
        transport.set_onmessage(onmessage_callback.as_ref().unchecked_ref());
        onmessage_callback.forget();

        let onopen_callback = Closure::<dyn FnMut(_)>::new(move |_: Event| {
            log::info!("Connected to host");
        });
        transport.set_onopen(onopen_callback.as_ref().unchecked_ref());
        onopen_callback.forget();

        Ok(Self {
            transport,
            message_id: AtomicU64::new(0),
            pending_messages,
            notification_tx,
            timeout,
        })
    }

    /// Sends a new request.
    ///
    /// Returns a oneshot channel to wait for the response.
    pub async fn send_message<P: Serialize>(
        &self,
        method: String,
        params: Option<P>,
    ) -> Result<Response<Value, Value>> {
        let req_id = self.next_message_id();
        let req = JsonRpc::new_request(Some(req_id), method, params);
        let req = serde_json::to_string(&req)?;
        self.transport.send(&req)?;

        // Create oneshot channel to wait for response
        let (tx, rx) = oneshot::channel::<Response<Value, Value>>();

        // Add to pending messages
        self.pending_messages
            .lock()
            .unwrap() //FIXME
            .insert(req_id, tx);

        let res = enforce_timeout(self.timeout, rx).await??;
        Ok(res)
    }

    /// Creates a notification receiver for a given method.
    ///
    /// Returns a stream of params of the notifications received.
    pub fn get_notification_receiver<T>(&self, method: String) -> impl Stream<Item = Result<T>>
    where
        T: DeserializeOwned + 'static,
    {
        // Create stream to handle notifications
        self.get_notification_rx()
            .filter_map(|message| {
                future::ready(match message {
                    Ok(message) => Some(message),
                    Err(_) => None, //TODO: handle lagged receiver?
                })
            })
            .filter(move |message| future::ready(message.method.as_str() == method))
            .map(|message| {
                let data = message
                    .params
                    .ok_or(JsonRpcError::NotificationWithoutParams(
                        message.method.as_str().into(),
                    ))?;
                let actual_message: T = serde_json::from_value(data)?;
                Ok(actual_message)
            })
    }

    /// Creates a notification sender.
    ///
    /// Returns a sink that consumes `Request` messages and sends them to the host.
    pub fn get_notification_sender<T>(&self) -> impl Sink<Request<T>, Error = anyhow::Error>
    where
        T: Serialize + 'static,
    {
        let (tx, mut rx) = mpsc::unbounded::<Request<T>>();
        let raw_transport = self.transport.get_raw();

        wasm_bindgen_futures::spawn_local(async move {
            while let Some(req) = rx.next().await {
                let req = JsonRpc::new_request(None, req.method.as_str().into(), req.params);
                let Ok(req) = serde_json::to_string(&req) else { continue };
                raw_transport.send_with_str(&req).unwrap(); //FIXME
            }
        });

        tx.sink_err_into()
    }

    /// Returns message id to create a request and increases
    /// internal counter by 1.
    fn next_message_id(&self) -> u64 {
        self.message_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Returns a new notification rx channel
    fn get_notification_rx(&self) -> tokio_stream::wrappers::BroadcastStream<Request<Value>> {
        tokio_stream::wrappers::BroadcastStream::new(self.notification_tx.subscribe())
    }

    /// Creates a new request.
    pub fn new_request<P: Serialize>(
        id: Option<u64>,
        method: String,
        params: Option<P>,
    ) -> Request<P> {
        let id = id.map(Id::Num);
        Request {
            id,
            jsonrpc: Version::V2,
            method: StrBuf::from_str(&method),
            params,
        }
    }
}
