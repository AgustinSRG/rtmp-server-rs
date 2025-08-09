// Control client status

use std::{collections::HashMap, sync::Arc};

use futures_util::{stream::SplitSink, SinkExt};
use tokio::{net::TcpStream, sync::{mpsc::Sender, Mutex}};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::{Message, Utf8Bytes};

use crate::{log::Logger, log_error};

use super::{ControlKeyValidationResponse, ControlServerMessage};

type ControlClientMessageSender = Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>;

/// Status of the control client
pub struct ControlClientStatus {
    /// Connected?
    pub connected: bool,

    /// The message sender
    pub msg_sender: Option<ControlClientMessageSender>,

    /// Key validation request counter
    pub request_count: u64,

    /// Pending key validation requests
    pub pending_requests: HashMap<u64, Sender<ControlKeyValidationResponse>>,
}

impl ControlClientStatus {
    /// Creates new ControlClientStatus
    pub fn new() -> ControlClientStatus {
        ControlClientStatus {
            connected: false,
            msg_sender: None,
            request_count: 0,
            pending_requests: HashMap::new(),
        }
    }

    /// Sets the status to connected
    pub async fn set_connected(
        status: &Mutex<ControlClientStatus>,
        msg_sender: ControlClientMessageSender,
    ) {
        let mut status_v = status.lock().await;

        status_v.connected = true;
        status_v.msg_sender = Some(msg_sender);
    }

    /// Sets the status to disconnected
    pub async fn set_disconnected(status: &Mutex<ControlClientStatus>) {
        let mut status_v = status.lock().await;

        status_v.connected = false;
        status_v.msg_sender = None;
    }

    /// Sends a message
    pub async fn send_message(
        status: &Mutex<ControlClientStatus>,
        message: ControlServerMessage,
        logger: &Logger,
    ) -> bool {
        let status_v = status.lock().await;
        if !status_v.connected {
            return false;
        }

        let msg_sender = match &status_v.msg_sender {
            Some(ms) => ms.clone(),
            None => {
                return false;
            }
        };

        drop(status_v);

        let mut msg_sender_v = msg_sender.lock().await;

        let msg_serialized = message.serialize();

        if logger.config.trace_enabled {
            logger.log_trace(&format!("SENT MESSAGE: {}", &msg_serialized));
        }

        match msg_sender_v
            .send(tungstenite::Message::Text(Utf8Bytes::from(
                message.serialize(),
            )))
            .await
        {
            Ok(_) => true,
            Err(e) => {
                log_error!(logger, &format!("Could not send a message: {}", e));

                false
            }
        }
    }

    /// Adds a key validation request, returning its ID
    pub async fn add_request(status: &Mutex<ControlClientStatus>, response_sender: Sender<ControlKeyValidationResponse>) -> Option<u64> {
        let mut status_v = status.lock().await;

        if !status_v.connected {
            drop(status_v);

            _ = response_sender.send(ControlKeyValidationResponse::Rejected).await;

            return None;
        }

        status_v.request_count += 1;

        let req_id = status_v.request_count;

        status_v.pending_requests.insert(req_id, response_sender);

        Some(req_id)
    }

    /// Completes pending key validation request
    pub async fn complete_request(status: &Mutex<ControlClientStatus>, id: u64, response: ControlKeyValidationResponse) {
        let mut status_v = status.lock().await;

        if let Some(rs) = status_v.pending_requests.get_mut(&id) {

            let response_sender = rs.clone();
            status_v.pending_requests.remove(&id);
            drop(status_v);

            _ = response_sender.send(response).await;
        }
    }


    /// Clears and rejects all pending requests
    pub async fn clear_pending_requests(status: &Mutex<ControlClientStatus>) {
        let mut status_v = status.lock().await;

        for response_sender in status_v.pending_requests.values() {
            _ = response_sender.send(ControlKeyValidationResponse::Rejected).await;
        }

        status_v.pending_requests.clear();
    }
}
