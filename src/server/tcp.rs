// TCP server

use std::{net::IpAddr, sync::Arc};

use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::{mpsc::Sender, Mutex},
};

use crate::{log::Logger, log_error, log_info};

use super::{handle_connection, RtmpServerContextExtended};

/// Run the TCP server
pub fn tcp_server(
    logger: Arc<Logger>,
    server_context: RtmpServerContextExtended,
    end_notifier: Sender<()>,
) {
    tokio::spawn(async move {
        let listen_addr = server_context.config.get_tcp_listen_addr();

        // Create listener
        let listener = match TcpListener::bind(&listen_addr).await {
            Ok(l) => l,
            Err(e) => {
                log_error!(logger, format!("Could not create TCP listener: {}", e));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        };

        log_info!(logger, format!("Listening on {}", listen_addr));

        loop {
            let accept_res = listener.accept().await;

            match accept_res {
                Ok((connection, addr)) => {
                    // Handle connection
                    handle_connection_tcp(
                        logger.clone(),
                        server_context.clone(),
                        connection,
                        addr.ip(),
                    );
                }
                Err(e) => {
                    log_error!(logger, format!("Could not accept connection: {}", e));
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

/// Handles a TCP connection, spawning a task for it
fn handle_connection_tcp(
    logger: Arc<Logger>,
    server_context: RtmpServerContextExtended,
    mut connection: TcpStream,
    ip: IpAddr,
) {
    tokio::spawn(async move {
        let is_exempted = server_context
            .config
            .as_ref()
            .max_concurrent_connections_whitelist
            .contains_ip(&ip);
        let mut should_accept = true;

        if !is_exempted {
            let mut ip_counter_v = server_context.ip_counter.as_ref().lock().await;
            should_accept = (*ip_counter_v).add(&ip);
            drop(ip_counter_v);
        }

        if should_accept {
            // Handle connection
            let (mut read_stream, write_stream) = connection.into_split();
            let write_stream_mu = Arc::new(Mutex::new(write_stream));

            handle_connection(
                logger,
                server_context.clone(),
                &mut read_stream,
                write_stream_mu.clone(),
                ip,
            )
            .await;

            // Ensure connection is closed
            let mut write_stream_mu_v = write_stream_mu.lock().await;
            let _ = (*write_stream_mu_v).shutdown().await;
            drop(write_stream_mu_v);

            // After connection is closed, remove from ip counter
            if !is_exempted {
                let mut ip_counter_v = server_context.ip_counter.as_ref().lock().await;
                (*ip_counter_v).remove(&ip);
                drop(ip_counter_v);
            }
        } else {
            log_info!(
                logger,
                format!("Rejected request from {} due to connection limit", ip)
            );
            let _ = connection.shutdown().await;
        }
    });
}
