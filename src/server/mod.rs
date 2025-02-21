// RTMP server

mod status;
mod config;
mod connection_handle;
mod context;
mod ip_count;
mod session_id_generator;
mod tcp;
mod tls;
mod utils;

use std::sync::Arc;

pub use status::*;
pub use config::*;
pub use connection_handle::*;
pub use context::*;
pub use ip_count::*;
pub use session_id_generator::*;
pub use tcp::*;
pub use tls::*;
pub use utils::*;
use tokio::sync::Mutex;

use crate::log::Logger;

/// Runs the RTMP server
pub async fn run_server(logger: Logger, server_context: RtmpServerContext) {
    let ip_counter = Arc::new(Mutex::new(IpConnectionCounter::new(
        server_context.config.as_ref(),
    )));
    let session_id_generator = Arc::new(Mutex::new(SessionIdGenerator::new()));

    let extended_context = RtmpServerContextExtended {
        config: server_context.config.clone(),
        status: server_context.status,
        control_key_validator_sender: server_context.control_key_validator_sender,
        ip_counter,
        session_id_generator,
    };

    let (end_notifier_tcp, mut end_receiver_tcp) = tokio::sync::mpsc::channel::<()>(1);

    tcp_server(
        Arc::new(logger.make_child_logger("[SERVER:TCP] ")),
        extended_context.clone(),
        end_notifier_tcp,
    );

    if server_context.config.tls.is_enabled() {
        let (end_notifier_tls, mut end_receiver_tls) = tokio::sync::mpsc::channel::<()>(1);

        tls_server(
            Arc::new(logger.make_child_logger("[SERVER:TLS] ")),
            extended_context.clone(),
            end_notifier_tls,
        );

        end_receiver_tls
            .recv()
            .await
            .expect("could not receive signal from TLS server thread");
    }

    end_receiver_tcp
        .recv()
        .await
        .expect("could not receive signal from TCP server thread");
}
