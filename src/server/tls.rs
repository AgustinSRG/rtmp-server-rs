// TCP server

use std::net::IpAddr;
use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::{net::TcpListener, sync::mpsc::Sender};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

use tokio_rustls::server::TlsStream;
use tokio_rustls::{rustls, TlsAcceptor};

use crate::log::Logger;

use super::{
    handle_connection, IpConnectionCounter, RtmpServerConfiguration, RtmpServerStatus,
    SessionIdGenerator,
};

/// Run the TCP server
pub fn tls_server(
    logger: Arc<Logger>,
    config: Arc<RtmpServerConfiguration>,
    server_status: Arc<Mutex<RtmpServerStatus>>,
    ip_counter: Arc<Mutex<IpConnectionCounter>>,
    session_id_generator: Arc<Mutex<SessionIdGenerator>>,
    end_notifier: Sender<()>,
) {
    tokio::spawn(async move {
        let certs_res = CertificateDer::pem_file_iter(&config.tls.certificate);
        let mut certificate: Vec<CertificateDer<'_>> = Vec::new();

        match certs_res {
            Ok(certs_iter) => {
                for c in certs_iter {
                    if let Ok(cert) = c {
                        certificate.push(cert);
                    }
                }
            }
            Err(e) => {
                logger.log_error(&format!("Could not load certificate: {}", e.to_string()));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        }

        let key_res = PrivateKeyDer::from_pem_file(&config.tls.key);
        let key: PrivateKeyDer<'_>;

        match key_res {
            Ok(k) => {
                key = k;
            }
            Err(e) => {
                logger.log_error(&format!("Could not load private key: {}", e.to_string()));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        }

        let listen_addr = config.tls.get_tcp_listen_addr();

        let tls_config_res = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certificate, key);
        let tls_config: rustls::ServerConfig;

        match tls_config_res {
            Ok(c) => {
                tls_config = c;
            }
            Err(e) => {
                logger.log_error(&format!(
                    "Could not load TLS configuration: {}",
                    e.to_string()
                ));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        }

        let acceptor = TlsAcceptor::from(Arc::new(tls_config));

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
                    handle_connection_tls(
                        acceptor.clone(),
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

fn handle_connection_tls(
    acceptor: TlsAcceptor,
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
            let stream_res = acceptor.accept(connection).await;
            let mut stream: TlsStream<TcpStream>;

            match stream_res {
                Ok(s) => {
                    stream = s;
                }
                Err(e) => {
                    logger
                        .as_ref()
                        .log_debug(&format!("Could not accept connection: {}", e.to_string()));
                    return;
                }
            }

            // Handle connection
            let (mut read_stream, write_stream) = tokio::io::split(stream);

            let write_stream_mu = Arc::new(Mutex::new(write_stream));

            handle_connection(
                &mut read_stream,
                write_stream_mu.clone(),
                ip,
                config.clone(),
                server_status,
                session_id_generator,
                logger.clone(),
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
