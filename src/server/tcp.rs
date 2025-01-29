// TCP server

use std::{net::IpAddr, sync::Arc};

use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::{mpsc::Sender, Mutex},
};

use crate::log::Logger;

use super::{
    handle_connection, IpConnectionCounter, RtmpServerConfiguration, RtmpServerStatus,
    SessionIdGenerator,
};

/// Run the TCP server
pub fn tcp_server(
    logger: Arc<Logger>,
    config: Arc<RtmpServerConfiguration>,
    server_status: Arc<Mutex<RtmpServerStatus>>,
    ip_counter: Arc<Mutex<IpConnectionCounter>>,
    session_id_generator: Arc<Mutex<SessionIdGenerator>>,
    end_notifier: Sender<()>,
) {
    tokio::spawn(async move {
        let listen_addr = config.get_tcp_listen_addr();

        // Create listener
        let listener_res = TcpListener::bind(&listen_addr).await;
        let listener: TcpListener;

        match listener_res {
            Ok(l) => {
                listener = l;
            }
            Err(e) => {
                logger.log_error(&format!("Could not create TCP listener: {}", e.to_string()));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        }

        logger.log_info(&format!("Listening on {}", listen_addr));

        loop {
            let accept_res = listener.accept().await;

            match accept_res {
                Ok((connection, addr)) => {
                    // Handle connection
                    handle_connection_tcp(
                        connection,
                        addr.ip(),
                        config.clone(),
                        server_status.clone(),
                        ip_counter.clone(),
                        session_id_generator.clone(),
                        logger.clone(),
                    );
                }
                Err(e) => {
                    logger.log_error(&format!("Could not accept connection: {}", e.to_string()));
                    end_notifier
                        .send(())
                        .await
                        .expect("failed to notify to main thread");
                    return;
                }
            }
        }
    });
}

fn handle_connection_tcp(
    mut connection: TcpStream,
    ip: IpAddr,
    config: Arc<RtmpServerConfiguration>,
    server_status: Arc<Mutex<RtmpServerStatus>>,
    ip_counter: Arc<Mutex<IpConnectionCounter>>,
    session_id_generator: Arc<Mutex<SessionIdGenerator>>,
    logger: Arc<Logger>,
) {
    tokio::spawn(async move {
        let is_exempted = config
            .as_ref()
            .max_concurrent_connections_whitelist
            .contains_ip(&ip);
        let mut should_accept = true;

        if !is_exempted {
            let mut ip_counter_v = ip_counter.as_ref().lock().await;
            should_accept = (*ip_counter_v).add(&ip);
            drop(ip_counter_v);
        }

        if should_accept {
            // Handle connection
            let (mut read_stream, write_stream) = connection.into_split();
            let write_stream_mu = Arc::new(Mutex::new(write_stream));

            handle_connection(
                &mut read_stream,
                write_stream_mu.clone(),
                ip,
                config.clone(),
                server_status,
                session_id_generator,
                logger,
            )
            .await;

            // Ensure connection is closed
            let mut write_stream_mu_v = write_stream_mu.lock().await;
            let _ = (*write_stream_mu_v).shutdown().await;
            drop(write_stream_mu_v);

            // After connection is closed, remove from ip counter
            if !is_exempted {
                let mut ip_counter_v = ip_counter.as_ref().lock().await;
                (*ip_counter_v).remove(&ip);
                drop(ip_counter_v);
            }
        } else {
            if config.log_requests {
                logger.as_ref().log_info(&format!(
                    "Rejected request from {} due to connection limit",
                    ip.to_string()
                ));
            }
            let _ = connection.shutdown().await;
        }
    });
}
