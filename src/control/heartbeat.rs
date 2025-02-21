// Logic to send heartbeat messages

use std::{sync::Arc, time::Duration};

use tokio::sync::{mpsc::Receiver, Mutex};

use crate::{control::ControlServerMessage, log::Logger};

use super::ControlClientStatus;

const HEARTBEAT_INTERVAL_SECONDS: u64 = 20;

/// Spawns a task to send heartbeat messages
/// 
/// # Arguments
/// 
/// * `logger` - The logger
/// * `status` - The control client status
/// * `cancel_receiver` - Receiver to listen for cancellation of the task
pub fn spawn_task_control_client_heartbeat(
    logger: Arc<Logger>,
    status: Arc<Mutex<ControlClientStatus>>,
    mut cancel_receiver: Receiver<()>,
) {
    tokio::spawn(async move {
        let mut finished = false;

        while !finished {
            // Wait
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECONDS)) => {}
                _ = cancel_receiver.recv() => {
                    finished = true;
                    continue;
                }
            }

            // Send heartbeat

            _ = ControlClientStatus::send_message(
                &status,
                ControlServerMessage::new("HEARTBEAT".to_string()),
                &logger,
            )
            .await;
        }
    });
}
