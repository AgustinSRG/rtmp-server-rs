// TCP server

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use filetime::FileTime;
use rustls::server::ResolvesServerCert;
use rustls::sign::{CertifiedKey, SigningKey};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;
use tokio::{net::TcpListener, sync::mpsc::Sender};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::{rustls, TlsAcceptor};

use crate::log::Logger;

use super::{handle_connection, RtmpServerConfiguration, RtmpServerContextExtended};

/// Run the TCP server
pub fn tls_server(
    logger: Arc<Logger>,
    server_context: RtmpServerContextExtended,
    end_notifier: Sender<()>,
) {
    tokio::spawn(async move {
        let cert_file_metadata =
            match tokio::fs::metadata(&server_context.config.tls.certificate).await {
                Ok(m) => m,
                Err(e) => {
                    logger.log_error(&format!("Could not load certificate: {}", e));
                    end_notifier
                        .send(())
                        .await
                        .expect("failed to notify to main thread");
                    return;
                }
            };

        let cert_file_mod_time =
            FileTime::from_last_modification_time(&cert_file_metadata).unix_seconds();

        let certs_res = CertificateDer::pem_file_iter(&server_context.config.tls.certificate);
        let mut certificate: Vec<CertificateDer<'_>> = Vec::new();

        match certs_res {
            Ok(certs_iter) => {
                for c in certs_iter.flatten() {
                    certificate.push(c);
                }
            }
            Err(e) => {
                logger.log_error(&format!("Could not load certificate: {}", e));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        }

        let key_file_metadata = match tokio::fs::metadata(&server_context.config.tls.key).await {
            Ok(m) => m,
            Err(e) => {
                logger.log_error(&format!("Could not load private key: {}", e));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        };

        let key_file_mod_time =
            FileTime::from_last_modification_time(&key_file_metadata).unix_seconds();

        let key = match PrivateKeyDer::from_pem_file(&server_context.config.tls.key) {
            Ok(k) => k,
            Err(e) => {
                logger.log_error(&format!("Could not load private key: {}", e));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        };

        let listen_addr = server_context.config.tls.get_tcp_listen_addr();

        let tls_config_builder = rustls::ServerConfig::builder();

        let key_provider = tls_config_builder.crypto_provider().key_provider;

        let signing_key = match key_provider.load_private_key(key) {
            Ok(k) => k,
            Err(e) => {
                logger.log_error(&format!("Could not load private key: {}", e));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        };

        let cert_resolver = Arc::new(CustomCertResolver::new(certificate, signing_key));

        let tls_config = tls_config_builder
            .with_no_client_auth()
            .with_cert_resolver(cert_resolver.clone());

        let acceptor = TlsAcceptor::from(Arc::new(tls_config));

        // Create listener
        let listener = match TcpListener::bind(&listen_addr).await {
            Ok(l) => l,
            Err(e) => {
                logger.log_error(&format!("Could not create TCP listener: {}", e));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        };

        logger.log_info(&format!("Listening on {}", listen_addr));

        // Spawn task to reload certificates periodically

        let cancel_tls_reloader_sender = if server_context.config.tls.check_reload_seconds > 0 {
            let (cancel_sender, cancel_receiver) = tokio::sync::mpsc::channel::<()>(1);

            spawn_task_periodically_reload_tls_config(
                logger.clone(),
                server_context.config.clone(),
                cert_resolver,
                cancel_receiver,
                cert_file_mod_time,
                key_file_mod_time,
            );

            Some(cancel_sender)
        } else {
            None
        };

        // Main loop

        loop {
            let accept_res = listener.accept().await;

            match accept_res {
                Ok((connection, addr)) => {
                    // Handle connection
                    handle_connection_tls(
                        logger.clone(),
                        server_context.clone(),
                        acceptor.clone(),
                        connection,
                        addr.ip(),
                    );
                }
                Err(e) => {
                    logger.log_error(&format!("Could not accept connection: {}", e));
                    end_notifier
                        .send(())
                        .await
                        .expect("failed to notify to main thread");
                    break;
                }
            }
        }

        if let Some(cancel_sender) = cancel_tls_reloader_sender {
            _ = cancel_sender.send(());
        }
    });
}

/// Handles a TLS connection
fn handle_connection_tls(
    logger: Arc<Logger>,
    server_context: RtmpServerContextExtended,
    tls_acceptor: TlsAcceptor,
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
            let stream = match tls_acceptor.accept(connection).await {
                Ok(s) => s,
                Err(e) => {
                    logger
                        .as_ref()
                        .log_debug(&format!("Could not accept connection: {}", e));
                    return;
                }
            };

            // Handle connection
            let (mut read_stream, write_stream) = tokio::io::split(stream);

            let write_stream_mu = Arc::new(Mutex::new(write_stream));

            handle_connection(
                logger.clone(),
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
            if server_context.config.log_requests {
                logger.as_ref().log_info(&format!(
                    "Rejected request from {} due to connection limit",
                    ip
                ));
            }
            let _ = connection.shutdown().await;
        }
    });
}

/// Custom certificate resolver
#[derive(Debug)]
struct CustomCertResolver {
    /// Key + certs
    pub certified_key: std::sync::Mutex<Arc<CertifiedKey>>,
}

impl CustomCertResolver {
    /// Creates new CustomCertResolver
    pub fn new(cert: Vec<CertificateDer<'static>>, key: Arc<dyn SigningKey>) -> CustomCertResolver {
        CustomCertResolver {
            certified_key: std::sync::Mutex::new(Arc::new(CertifiedKey::new(cert, key))),
        }
    }

    /// Sets TLS configuration
    pub fn set_config(&self, cert: Vec<CertificateDer<'static>>, key: Arc<dyn SigningKey>) {
        let mut certified_key_v = self.certified_key.lock().unwrap();
        *certified_key_v = Arc::new(CertifiedKey::new(cert, key));
    }
}

impl ResolvesServerCert for CustomCertResolver {
    fn resolve(
        &self,
        _client_hello: rustls::server::ClientHello<'_>,
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        let certified_key_v = self.certified_key.lock().unwrap();
        Some(certified_key_v.clone())
    }
}

fn spawn_task_periodically_reload_tls_config(
    logger: Arc<Logger>,
    config: Arc<RtmpServerConfiguration>,
    cert_resolver: Arc<CustomCertResolver>,
    mut cancel_receiver: Receiver<()>,
    initial_cert_time: i64,
    initial_key_time: i64,
) {
    tokio::spawn(async move {
        let mut cert_time = initial_cert_time;
        let mut key_time = initial_key_time;

        let mut finished = false;

        while !finished {
            // Wait
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(config.tls.check_reload_seconds as u64)) => {}
                _ = cancel_receiver.recv() => {
                    finished = true;
                    continue;
                }
            }

            logger.log_debug("Checking for changes in TLS configuration...");

            // Check

            let cert_file_metadata = match tokio::fs::metadata(&config.tls.certificate).await {
                Ok(m) => m,
                Err(e) => {
                    logger.log_error(&format!("Could not load certificate: {}", e));
                    continue;
                }
            };

            let cert_file_mod_time =
                FileTime::from_last_modification_time(&cert_file_metadata).unix_seconds();

            let key_file_metadata = match tokio::fs::metadata(&config.tls.key).await {
                Ok(m) => m,
                Err(e) => {
                    logger.log_error(&format!("Could not load private key: {}", e));
                    continue;
                }
            };

            let key_file_mod_time =
                FileTime::from_last_modification_time(&key_file_metadata).unix_seconds();

            if cert_file_mod_time == cert_time && key_file_mod_time == key_time {
                logger.log_debug("No changes detected in TLS configuration");

                continue;
            }

            // Changes detected, reload configuration

            let certs_res = CertificateDer::pem_file_iter(&config.tls.certificate);
            let mut certificate: Vec<CertificateDer<'_>> = Vec::new();

            match certs_res {
                Ok(certs_iter) => {
                    for c in certs_iter.flatten() {
                        certificate.push(c);
                    }
                }
                Err(e) => {
                    logger.log_error(&format!("Could not load certificate: {}", e));
                    continue;
                }
            }

            let key = match PrivateKeyDer::from_pem_file(&config.tls.key) {
                Ok(k) => k,
                Err(e) => {
                    logger.log_error(&format!("Could not load private key: {}", e));
                    continue;
                }
            };

            let tls_config_builder = rustls::ServerConfig::builder();

            let key_provider = tls_config_builder.crypto_provider().key_provider;

            let signing_key = match key_provider.load_private_key(key) {
                Ok(k) => k,
                Err(e) => {
                    logger.log_error(&format!("Could not load private key: {}", e));
                    continue;
                }
            };

            // Update mod times
            cert_time = cert_file_mod_time;
            key_time = key_file_mod_time;

            // Update config
            cert_resolver.set_config(certificate, signing_key);

            // Log
            logger.log_info("TLS configuration reloaded");
        }
    });
}
