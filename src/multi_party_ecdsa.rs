use crate::{
    client::json_rpc::JsonRpc,
    utils::serializer::{
        deserialize_any_from_js, serialize_any_to_js, serialize_response_to_js,
        serialize_str_error_to_js,
    },
};
use anyhow::{Context, Result};
use curv::{arithmetic::Converter, elliptic::curves::Secp256k1, BigInt};
use futures::{future, pin_mut, SinkExt, Stream, StreamExt, TryStreamExt};
use gloo_utils::format::JsValueSerdeExt;
use mpc_manager::{
    service::{
        group_service::{GroupCreateRequest, GroupJoinRequest, GroupMethod},
        session_service::{
            SessionCreateRequest, SessionCreatedNotification, SessionEvent, SessionLoginRequest,
            SessionMessageNotification, SessionMessageRequest, SessionMethod,
            SessionReadyNotification, SessionSignupRequest,
        },
    },
    state::{parameters::Parameters, session::SessionKind},
};
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::{
    keygen::{Keygen, LocalKey, ProtocolMessage},
    sign::{OfflineProtocolMessage, OfflineStage, PartialSignature, SignManual},
};
use round_based::AsyncProtocol;
use serde::Serialize;
use std::{
    collections::VecDeque,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};
use thiserror::Error;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;
use wasm_bindgen::{prelude::wasm_bindgen, JsError, JsValue};

mod types;

type ProtocolMessageNotification = SessionMessageNotification<round_based::Msg<ProtocolMessage>>;
type OfflineProtocolMessageNotification =
    SessionMessageNotification<round_based::Msg<OfflineProtocolMessage>>;
type PartialSignatureNotification = SessionMessageNotification<round_based::Msg<PartialSignature>>;

#[derive(Debug, Error)]
pub enum MultiPartyEcdsaError {
    #[error("invalid group id `${0}`")]
    InvalidGroupId(String),
    #[error("invalid session id `${0}`")]
    InvalidSessionId(String),
    #[error("invalid local key")]
    InvalidLocalKey,
    #[error("protocol execution failed")]
    FailedProtocolExecution(String), //FIXME: should implement with #[source]
}

#[derive(Default)]
struct PendingMessages {
    protocol_messages: Mutex<VecDeque<ProtocolMessageNotification>>,
    offline_protocol_messages: Mutex<VecDeque<OfflineProtocolMessageNotification>>,
    partial_signature_messages: Mutex<VecDeque<PartialSignatureNotification>>,
}

struct MessageChannels {
    protocol_message_tx: broadcast::Sender<ProtocolMessageNotification>,
    offline_protocol_message_tx: broadcast::Sender<OfflineProtocolMessageNotification>,
    partial_signature_message_tx: broadcast::Sender<PartialSignatureNotification>,
}

#[wasm_bindgen]
pub struct MultiPartyEcdsa {
    json_rpc: JsonRpc,
    pending_messages: Arc<PendingMessages>,
    message_channels: MessageChannels,
}

#[wasm_bindgen]
impl MultiPartyEcdsa {
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str, timeout_in_ms: Option<u32>) -> Self {
        let timeout = timeout_in_ms.map(|t| Duration::from_millis(t.into()));
        let json_rpc = JsonRpc::new(url.into(), timeout).unwrap();
        let pending_messages = Arc::new(PendingMessages::default());
        let message_channels = MessageChannels {
            protocol_message_tx: broadcast::channel::<ProtocolMessageNotification>(32).0,
            offline_protocol_message_tx: broadcast::channel::<OfflineProtocolMessageNotification>(
                32,
            )
            .0,
            partial_signature_message_tx: broadcast::channel::<PartialSignatureNotification>(32).0,
        };

        let mut incoming_messages = json_rpc.get_notification_receiver::<serde_json::Value>(
            SessionMethod::SessionMessage.to_string(),
        );

        let pending_messages_c = pending_messages.clone();
        let protocol_message_tx = message_channels.protocol_message_tx.clone();
        let offline_protocol_message_tx = message_channels.offline_protocol_message_tx.clone();
        let partial_signature_message_tx = message_channels.partial_signature_message_tx.clone();

        wasm_bindgen_futures::spawn_local(async move {
            while let Some(Ok(message)) = incoming_messages.next().await {
                if let Ok(message) =
                    serde_json::from_value::<ProtocolMessageNotification>(message.clone())
                {
                    if protocol_message_tx.send(message.clone()).is_err() {
                        pending_messages_c
                            .protocol_messages
                            .lock()
                            .unwrap()
                            .push_back(message.clone())
                    }
                    continue;
                }
                if let Ok(message) =
                    serde_json::from_value::<OfflineProtocolMessageNotification>(message.clone())
                {
                    if offline_protocol_message_tx.send(message.clone()).is_err() {
                        pending_messages_c
                            .offline_protocol_messages
                            .lock()
                            .unwrap()
                            .push_back(message.clone())
                    }
                    continue;
                }
                if let Ok(message) = serde_json::from_value::<PartialSignatureNotification>(message)
                {
                    if partial_signature_message_tx.send(message.clone()).is_err() {
                        pending_messages_c
                            .partial_signature_messages
                            .lock()
                            .unwrap()
                            .push_back(message)
                    }
                    continue;
                }
            }
        });

        Self {
            json_rpc,
            pending_messages,
            message_channels,
        }
    }

    #[wasm_bindgen(js_name = "groupCreate")]
    pub async fn group_create(
        &mut self,
        parties: u16,
        threshold: u16,
    ) -> Result<types::GroupCreateResponse, JsError> {
        log::info!("Creating group");
        let res = self
            .json_rpc
            .send_message(
                GroupMethod::GroupCreate.to_string(),
                Some(GroupCreateRequest {
                    parameters: Parameters::new(parties, threshold)
                        .map_err(serialize_str_error_to_js)?,
                }),
            )
            .await
            .map_err(serialize_str_error_to_js)?;
        serialize_response_to_js(res).map(|val| val.into())
    }

    #[wasm_bindgen(js_name = "groupJoin")]
    pub async fn group_join(
        &mut self,
        group_id: &str,
    ) -> Result<types::GroupJoinResponse, JsError> {
        log::info!("Joining group with group_id {}", group_id);
        let group_id = Uuid::try_from(group_id).map_err(serialize_str_error_to_js)?;
        let res = self
            .json_rpc
            .send_message(
                GroupMethod::GroupJoin.to_string(),
                Some(GroupJoinRequest { group_id }),
            )
            .await
            .map_err(serialize_str_error_to_js)?;
        serialize_response_to_js(res).map(|val| val.into())
    }

    #[wasm_bindgen(js_name = "sessionCreate")]
    pub async fn session_create(
        &mut self,
        group_id: &str,
        kind: types::SessionKind,
        value: JsValue,
    ) -> Result<types::SessionCreateResponse, JsError> {
        log::info!("Creating session with group_id {}", group_id);
        let group_id = Uuid::try_from(group_id).map_err(serialize_str_error_to_js)?;
        let kind =
            SessionKind::from_str(&kind.as_string().unwrap()).map_err(serialize_str_error_to_js)?;
        let value = value.into_serde().map_err(serialize_str_error_to_js)?;
        let res = self
            .json_rpc
            .send_message(
                SessionMethod::SessionCreate.to_string(),
                Some(SessionCreateRequest {
                    group_id,
                    kind,
                    value,
                }),
            )
            .await
            .map_err(serialize_str_error_to_js)?;
        serialize_response_to_js(res).map(|val| val.into())
    }

    #[wasm_bindgen(js_name = "sessionSignup")]
    pub async fn session_signup(
        &mut self,
        group_id: &str,
        session_id: &str,
    ) -> Result<types::SessionSignupResponse, JsError> {
        log::info!(
            "Signing up to session with group_id {} and session_id {}",
            group_id,
            session_id
        );
        let group_id = Uuid::try_from(group_id).map_err(serialize_str_error_to_js)?;
        let session_id = Uuid::try_from(session_id).map_err(serialize_str_error_to_js)?;
        let res = self
            .json_rpc
            .send_message(
                SessionMethod::SessionSignup.to_string(),
                Some(SessionSignupRequest {
                    group_id,
                    session_id,
                }),
            )
            .await
            .map_err(serialize_str_error_to_js)?;
        serialize_response_to_js(res).map(|val| val.into())
    }

    #[wasm_bindgen(js_name = "sessionLogin")]
    pub async fn session_login(
        &mut self,
        group_id: &str,
        session_id: &str,
        party_number: u16,
    ) -> Result<types::SessionSignupResponse, JsError> {
        log::info!(
            "Logging to session with group_id {}, session_id {} and party number {}",
            group_id,
            session_id,
            party_number
        );
        let group_id = Uuid::try_from(group_id).map_err(serialize_str_error_to_js)?;
        let session_id = Uuid::try_from(session_id).map_err(serialize_str_error_to_js)?;
        let res = self
            .json_rpc
            .send_message(
                SessionMethod::SessionLogin.to_string(),
                Some(SessionLoginRequest {
                    group_id,
                    session_id,
                    party_number,
                }),
            )
            .await
            .map_err(serialize_str_error_to_js)?;
        serialize_response_to_js(res).map(|val| val.into())
    }

    #[wasm_bindgen(js_name = "onSessionCreated")]
    pub fn on_session_created(&self, callback: js_sys::Function) {
        let mut incoming = self
            .json_rpc
            .get_notification_receiver::<SessionCreatedNotification>(
                SessionEvent::SessionCreated.to_string(),
            );

        wasm_bindgen_futures::spawn_local(async move {
            while let Some(msg) = incoming.next().await {
                let Ok(msg) = msg else { continue };
                let Ok(msg) = serialize_any_to_js(msg) else { continue };
                callback.call1(&JsValue::NULL, &msg).unwrap(); //FIXME
            }
        })
    }

    #[wasm_bindgen(js_name = "onSessionReady")]
    pub fn on_session_ready(&self, callback: js_sys::Function) {
        let mut incoming = self
            .json_rpc
            .get_notification_receiver::<SessionReadyNotification>(
                SessionEvent::SessionReady.to_string(),
            );

        wasm_bindgen_futures::spawn_local(async move {
            while let Some(msg) = incoming.next().await {
                let Ok(msg) = msg else { continue };
                let Ok(msg) = serialize_any_to_js(msg) else { continue };
                callback.call1(&JsValue::NULL, &msg).unwrap(); //FIXME
            }
        })
    }

    #[wasm_bindgen]
    pub async fn keygen(
        &mut self,
        group_id: &str,
        session_id: &str,
        party_number: u16,
        parties: u16,
        threshold: u16,
    ) -> Result<types::KeygenResponse, JsError> {
        log::info!(
            "Generating new key with group_id {}, session_id {} and party number {}",
            group_id,
            session_id,
            party_number
        );
        let group_id = Uuid::try_from(group_id)
            .map_err(|_| MultiPartyEcdsaError::InvalidGroupId(group_id.into()))?;
        let session_id = Uuid::try_from(session_id)
            .map_err(|_| MultiPartyEcdsaError::InvalidSessionId(session_id.into()))?;

        // Create channels for communication with async-protocol
        let incoming = self
            .get_protocol_message_receiver()
            .filter_map(|message| match message {
                Ok(message) => {
                    if !(message.group_id == group_id
                        && message.session_id == session_id
                        && message.sender != party_number)
                    {
                        return future::ready(None);
                    }
                    future::ready(Some(Ok::<_, anyhow::Error>(message.message)))
                }
                Err(err) => future::ready(Some(Err(err))),
            });
        let outgoing = self
            .json_rpc
            .get_notification_sender()
            .with::<_, _, _, anyhow::Error>(|message: round_based::Msg<ProtocolMessage>| {
                let params = SessionMessageRequest {
                    group_id,
                    session_id,
                    message: message.clone(),
                    receiver: message.receiver,
                };
                future::ready(Ok(JsonRpc::new_request(
                    None,
                    SessionMethod::SessionMessage.to_string(),
                    Some(params),
                )))
            });

        let keygen =
            Keygen::new(party_number, threshold, parties).map_err(serialize_str_error_to_js)?;

        let incoming = incoming.fuse();
        pin_mut!(incoming);
        pin_mut!(outgoing);

        let local_key = AsyncProtocol::new(keygen, incoming, outgoing)
            .run()
            .await
            .map_err(serialize_str_error_to_js)?;

        #[derive(Serialize)]
        struct KeygenResponse {
            #[serde(rename = "localKey")]
            local_key: LocalKey<Secp256k1>,
            #[serde(rename = "publicKey")]
            public_key: String,
        }

        let public_key = hex::encode(local_key.public_key().to_bytes(false).as_ref());
        let output = KeygenResponse {
            local_key,
            public_key,
        };

        serialize_any_to_js(&output).map(|val| val.into())
    }

    #[wasm_bindgen]
    pub async fn sign(
        &mut self,
        group_id: &str,
        session_id: &str,
        local_key: JsValue,
        parties: Vec<u16>,
        data_to_sign: &[u8],
    ) -> Result<types::SignResponse, JsError> {
        log::info!(
            "Signing data with group_id {}, session_id {} and parties {:?}",
            group_id,
            session_id,
            parties
        );
        let group_id = Uuid::try_from(group_id)
            .map_err(|_| MultiPartyEcdsaError::InvalidGroupId(group_id.into()))?;
        let session_id = Uuid::try_from(session_id)
            .map_err(|_| MultiPartyEcdsaError::InvalidSessionId(session_id.into()))?;
        let local_key: LocalKey<Secp256k1> = deserialize_any_from_js(local_key)
            .map_err(|_| MultiPartyEcdsaError::InvalidLocalKey)?;

        let party_number = local_key.i;
        let number_of_parties = parties.len();

        // Create channels for offline stage communication with async-protocol
        let incoming = self
            .get_offline_protocol_message_receiver()
            .try_filter(|message| {
                future::ready(
                    message.group_id == group_id
                        && message.session_id == session_id
                        && message.sender != party_number,
                )
            })
            .map_ok(|message| message.message);
        let outgoing = self
            .json_rpc
            .get_notification_sender()
            .with::<_, _, _, anyhow::Error>(|message: round_based::Msg<OfflineProtocolMessage>| {
                let params = SessionMessageRequest {
                    group_id,
                    session_id,
                    message: message.clone(),
                    receiver: message.receiver,
                };
                future::ready(Ok(JsonRpc::new_request(
                    None,
                    SessionMethod::SessionMessage.to_string(),
                    Some(params),
                )))
            });

        let incoming = incoming.fuse();
        pin_mut!(incoming);
        pin_mut!(outgoing);

        let signing = OfflineStage::new(party_number, parties, local_key)?;
        let completed_offline_stage = AsyncProtocol::new(signing, incoming, outgoing)
            .run()
            .await
            .map_err(|e| MultiPartyEcdsaError::FailedProtocolExecution(e.to_string()))?;

        // Create channels for online stage communication with async-protocol
        let incoming = self
            .get_partial_signature_message_receiver()
            .try_filter(|message| {
                future::ready(
                    message.group_id == group_id
                        && message.session_id == session_id
                        && message.sender != party_number,
                )
            })
            .map_ok(|message| message.message);
        let outgoing = self
            .json_rpc
            .get_notification_sender()
            .with::<_, _, _, anyhow::Error>(|message: round_based::Msg<PartialSignature>| {
                let params = SessionMessageRequest {
                    group_id,
                    session_id,
                    message: message.clone(),
                    receiver: message.receiver,
                };
                future::ready(Ok(JsonRpc::new_request(
                    None,
                    SessionMethod::SessionMessage.to_string(),
                    Some(params),
                )))
            });

        let incoming = incoming.fuse();
        pin_mut!(incoming);
        pin_mut!(outgoing);

        let (signing, partial_signature) =
            SignManual::new(BigInt::from_bytes(data_to_sign), completed_offline_stage)?;

        outgoing
            .send(round_based::Msg {
                sender: party_number,
                receiver: None,
                body: partial_signature,
            })
            .await
            .map_err(serialize_str_error_to_js)?;

        let partial_signatures: Vec<_> = incoming
            .take(number_of_parties - 1)
            .map_ok(|msg| msg.body)
            .try_collect()
            .await
            .map_err(serialize_str_error_to_js)?;
        let signature = signing
            .complete(&partial_signatures)
            .context("online stage failed")
            .map_err(serialize_str_error_to_js)?;

        serialize_any_to_js(&signature).map(|val| val.into())
    }

    fn get_protocol_message_receiver(
        &self,
    ) -> impl Stream<Item = Result<ProtocolMessageNotification>> {
        // Create receiver stream
        let receiver = BroadcastStream::new(self.message_channels.protocol_message_tx.subscribe())
            .map_err(|e| e.into());

        // Resend all pending messages
        let mut pending = self.pending_messages.protocol_messages.lock().unwrap();
        while pending.len() > 0 {
            let message = pending.pop_front().unwrap();
            self.message_channels
                .protocol_message_tx
                .send(message)
                .unwrap();
        }

        receiver
    }

    fn get_offline_protocol_message_receiver(
        &self,
    ) -> impl Stream<Item = Result<OfflineProtocolMessageNotification>> {
        // Create receiver stream
        let receiver = BroadcastStream::new(
            self.message_channels
                .offline_protocol_message_tx
                .subscribe(),
        )
        .map_err(|e| e.into());

        // Resend all pending messages
        let mut pending = self
            .pending_messages
            .offline_protocol_messages
            .lock()
            .unwrap();
        while pending.len() > 0 {
            let message = pending.pop_front().unwrap();
            self.message_channels
                .offline_protocol_message_tx
                .send(message)
                .unwrap();
        }

        receiver
    }

    fn get_partial_signature_message_receiver(
        &self,
    ) -> impl Stream<Item = Result<PartialSignatureNotification>> {
        // Create receiver stream
        let receiver = BroadcastStream::new(
            self.message_channels
                .partial_signature_message_tx
                .subscribe(),
        )
        .map_err(|e| e.into());

        // Resend all pending messages
        let mut pending = self
            .pending_messages
            .partial_signature_messages
            .lock()
            .unwrap();
        while pending.len() > 0 {
            let message = pending.pop_front().unwrap();
            self.message_channels
                .partial_signature_message_tx
                .send(message)
                .unwrap();
        }

        receiver
    }
}
