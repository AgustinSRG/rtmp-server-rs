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
        let cert_file_metadata = match tokio::fs::metadata(&config.tls.certificate).await {
            Ok(m) => m,
            Err(e) => {
                logger.log_error(&format!("Could not load certificate: {}", e.to_string()));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        };

        let cert_file_mod_time =
            FileTime::from_last_modification_time(&cert_file_metadata).unix_seconds();

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

        let key_file_metadata = match tokio::fs::metadata(&config.tls.key).await {
            Ok(m) => m,
            Err(e) => {
                logger.log_error(&format!("Could not load private key: {}", e.to_string()));
                end_notifier
                    .send(())
                    .await
                    .expect("failed to notify to main thread");
                return;
            }
        };

        let key_file_mod_time =
            FileTime::from_last_modification_time(&key_file_metadata).unix_seconds();

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

        let tls_config_builder = rustls::ServerConfig::builder();

        let key_provider = tls_config_builder.crypto_provider().key_provider;

        let signing_key = match key_provider.load_private_key(key) {
            Ok(k) => k,
            Err(e) => {
                logger.log_error(&format!("Could not load private key: {}", e.to_string()));
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

        // Spawn task to reload certificates periodically

        let cancel_tls_reloader_sender: Option<Sender<()>>;

        if config.tls.check_reload_seconds > 0 {
            let (cancel_sender, cancel_receiver) = tokio::sync::mpsc::channel::<()>(1);

            spawn_task_periodically_reload_tls_config(
                cert_file_mod_time,
                key_file_mod_time,
                logger.clone(),
                config.clone(),
                cert_resolver,
                cancel_receiver,
            );

            cancel_tls_reloader_sender = Some(cancel_sender);
        } else {
            cancel_tls_reloader_sender = None;
        }

        // Main loop

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
                    break;
                }
            }
        }

        if let Some(cancel_sender) = cancel_tls_reloader_sender {
            _ = cancel_sender.send(());
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
            let stream: TlsStream<TcpStream>;

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
    initial_cert_time: i64,
    initial_key_time: i64,
    logger: Arc<Logger>,
    config: Arc<RtmpServerConfiguration>,
    cert_resolver: Arc<CustomCertResolver>,
    mut cancel_receiver: Receiver<()>,
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

            if logger.config.debug_enabled {
                logger.log_debug("Checking for changes in TLS configuration...");
            }

            // Check

            let cert_file_metadata = match tokio::fs::metadata(&config.tls.certificate).await {
                Ok(m) => m,
                Err(e) => {
                    logger.log_error(&format!("Could not load certificate: {}", e.to_string()));
                    continue;
                }
            };

            let cert_file_mod_time =
                FileTime::from_last_modification_time(&cert_file_metadata).unix_seconds();

            let key_file_metadata = match tokio::fs::metadata(&config.tls.key).await {
                Ok(m) => m,
                Err(e) => {
                    logger.log_error(&format!("Could not load private key: {}", e.to_string()));
                    continue;
                }
            };

            let key_file_mod_time =
                FileTime::from_last_modification_time(&key_file_metadata).unix_seconds();

            if cert_file_mod_time == cert_time && key_file_mod_time == key_time {
                if logger.config.debug_enabled {
                    logger.log_debug("No changes detected in TLS configuration");
                }

                continue;
            }

            // Changes detected, reload configuration

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
                    continue;
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
                    continue;
                }
            }

            let tls_config_builder = rustls::ServerConfig::builder();

            let key_provider = tls_config_builder.crypto_provider().key_provider;

            let signing_key = match key_provider.load_private_key(key) {
                Ok(k) => k,
                Err(e) => {
                    logger.log_error(&format!("Could not load private key: {}", e.to_string()));
                    continue;
                }
            };

            // Update mod times
            cert_time = cert_file_mod_time;
            key_time = key_file_mod_time;

            // Update config
            cert_resolver.set_config(certificate, signing_key);

            // Log
            if logger.config.info_enabled {
                logger.log_debug("TLS configuration reloaded");
            }
        }
    });
}
