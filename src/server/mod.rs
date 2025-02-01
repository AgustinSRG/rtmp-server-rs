// RTMP server

mod config;
mod connection_handle;
mod ip_count;
mod session_id_generator;
mod status;
mod tls;
mod tcp;

use std::sync::Arc;

pub use config::*;
pub use connection_handle::*;
pub use ip_count::*;
pub use session_id_generator::*;
pub use status::*;
pub use tls::*;
pub use tcp::*;
use tokio::sync::{mpsc::Sender, Mutex};

use crate::{control::ControlKeyValidationRequest, log::Logger};

/// Runs the RTMP server
pub async fn run_server(logger: Logger, config: Arc<RtmpServerConfiguration>, server_status: Arc<Mutex<RtmpServerStatus>>, control_key_validator_sender: Option<Sender<ControlKeyValidationRequest>>) {
    let ip_counter = Arc::new(Mutex::new(IpConnectionCounter::new(config.as_ref())));
    let session_id_generator = Arc::new(Mutex::new(SessionIdGenerator::new()));

    let (end_notifier_tcp, mut end_receiver_tcp) = tokio::sync::mpsc::channel::<()>(1);

    tcp_server(Arc::new(logger.make_child_logger("[SERVER:TCP] ")), config.clone(), server_status.clone(), ip_counter.clone(), session_id_generator.clone(), control_key_validator_sender.clone(), end_notifier_tcp);

    if config.tls.is_enabled() {
        let (end_notifier_tls, mut end_receiver_tls) = tokio::sync::mpsc::channel::<()>(1);

        tls_server(Arc::new(logger.make_child_logger("[SERVER:TLS] ")), config.clone(), server_status.clone(), ip_counter.clone(), session_id_generator.clone(), control_key_validator_sender.clone(), end_notifier_tls);

        end_receiver_tls.recv().await.expect("could not receive signal from TLS server thread");
    }

    end_receiver_tcp.recv().await.expect("could not receive signal from TCP server thread");
}
