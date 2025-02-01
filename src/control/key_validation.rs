// Callback system to request key validation to the control server

use std::{collections::HashMap, net::IpAddr, sync::Arc};

use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

use crate::log::Logger;

use super::{ControlClientStatus, ControlServerMessage};

/// Size for the buffer of the channel to communicate key validation requests
pub const KEY_VALIDATION_CHANNEL_BUFFER_SIZE: usize = 16;

/// Response for key validation
pub enum ControlKeyValidationResponse {
    Accepted { stream_id: String },
    Rejected,
}

/// Request to validate stream keys against the control server
pub enum ControlKeyValidationRequest {
    PublishStart {
        /// The channel
        channel: String,

        /// The provided key to publish
        key: String,

        /// The IP of the publisher
        client_ip: String,

        /// Sender for the response
        response_sender: Sender<ControlKeyValidationResponse>,
    },
    PublishEnd {
        /// The channel
        channel: String,

        /// The stream_id
        stream_id: String,
    },
}

/// Validates a stream key against the control server
/// control_key_validator_sender - Sender to communicate with the control server
/// channel - Channel
/// key - Stream key
/// client_ip - IP of the publisher
/// Returns true if valid, false if invalid or error
pub async fn control_validate_key(
    control_key_validator_sender: &Sender<ControlKeyValidationRequest>,
    channel: &str,
    key: &str,
    client_ip: &IpAddr,
) -> Option<String> {
    // Create channel to communicate the response
    let (response_sender, mut response_receiver) =
        tokio::sync::mpsc::channel::<ControlKeyValidationResponse>(1);

    // Send the request

    if control_key_validator_sender
        .send(ControlKeyValidationRequest::PublishStart {
            channel: channel.to_string(),
            key: key.to_string(),
            client_ip: client_ip.to_string(),
            response_sender,
        })
        .await.is_err()
    {
        return None;
    }

    // Get the response

    match response_receiver.recv().await {
        Some(r) => match r {
            ControlKeyValidationResponse::Accepted { stream_id } => Some(stream_id),
            ControlKeyValidationResponse::Rejected => None,
        },
        None => None,
    }
}

/// Spawns task to handle key validations against the control server
/// logger- The logger
/// status - The client status
/// request_receiver - Receiver for the requests
pub fn spawn_task_handle_control_key_validations(
    logger: Arc<Logger>,
    status: Arc<Mutex<ControlClientStatus>>,
    mut request_receiver: Receiver<ControlKeyValidationRequest>,
) {
    tokio::spawn(async move {
        loop {
            let req = match request_receiver.recv().await {
                Some(m) => m,
                None => {
                    logger.log_error("Control key validation channel was closed");
                    return;
                }
            };

            match req {
                ControlKeyValidationRequest::PublishStart { channel, key, client_ip, response_sender } => {
                    if logger.config.debug_enabled {
                        logger.log_debug(&format!(
                            "Handling validation request for channel: {} and key: {}",
                            &channel, &key
                        ));
                    }
        
                    // Add request
        
                    let req_id = match ControlClientStatus::add_request(&status, response_sender).await
                    {
                        Some(id) => id,
                        None => {
                            if logger.config.debug_enabled {
                                logger.log_debug("Not connected to the control server, so the key validation request was rejected.");
                            }
        
                            return;
                        }
                    };
        
                    // Send message to the server
        
                    let mut parameters: HashMap<String, String> = HashMap::new();
        
                    parameters.insert("Request-ID".to_string(), req_id.to_string());
                    parameters.insert("Stream-Channel".to_string(), channel);
                    parameters.insert("Stream-Key".to_string(), key);
                    parameters.insert("User-IP".to_string(), client_ip);
        
                    let msg = ControlServerMessage::new_with_parameters(
                        "PUBLISH-REQUEST".to_string(),
                        parameters,
                    );
        
                    if !ControlClientStatus::send_message(&status, msg, &logger).await {
                        // Failed to send message, reject the request
                        ControlClientStatus::complete_request(
                            &status,
                            req_id,
                            ControlKeyValidationResponse::Rejected,
                        )
                        .await;
                    }
                },
                ControlKeyValidationRequest::PublishEnd { channel, stream_id } => {
                     // Send message to the server
        
                     let mut parameters: HashMap<String, String> = HashMap::new();
        
                     parameters.insert("Stream-Channel".to_string(), channel);
                     parameters.insert("Stream-ID".to_string(), stream_id);
         
                     let msg = ControlServerMessage::new_with_parameters(
                         "PUBLISH-END".to_string(),
                         parameters,
                     );
         
                     _ = ControlClientStatus::send_message(&status, msg, &logger).await;
                },
            }
        }
    });
}
