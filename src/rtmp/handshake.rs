// RTMP handshake utils

use hmac::{Hmac, Mac};
use sha2::Sha256;

use rand::{rngs::StdRng, RngCore, SeedableRng};

use std::sync::LazyLock;

use crate::{log::Logger, log_debug};

use super::{
    GENUINE_FMS, GENUINE_FP, MESSAGE_FORMAT_0, MESSAGE_FORMAT_1, MESSAGE_FORMAT_2, RANDOM_CRUD,
    RTMP_SIG_SIZE, RTMP_VERSION, SHA256DL, SHA256K,
};

// Consts for handshake

static GENUINE_FMS_PLUS_CRUD: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let str_bytes: Vec<u8> = GENUINE_FMS.bytes().collect();

    let mut v: Vec<u8> = vec![0; str_bytes.len() + RANDOM_CRUD.len()];

    v[0..str_bytes.len()].copy_from_slice(&str_bytes);
    v[str_bytes.len()..].copy_from_slice(RANDOM_CRUD);

    v
});

/// Generates full RTMP handshake response
/// client_signature - Client signature
/// logger - Logger of the RTMP session
pub fn generate_s0_s1_s2(client_signature: &[u8], logger: &Logger) -> Result<Vec<u8>, ()> {
    let msg_format = detect_client_message_format(client_signature, logger)?;

    let mut all_bytes: Vec<u8> = Vec::new();

    if msg_format == MESSAGE_FORMAT_0 {
        log_debug!(logger, "Using basic handshake");

        all_bytes.push(RTMP_VERSION);
        all_bytes.extend(client_signature);
        all_bytes.extend(client_signature);
    } else {
        log_debug!(logger, "Using S1S2 handshake");

        let s1 = generate_s1(msg_format, logger)?;
        let s2 = generate_s2(msg_format, client_signature, logger)?;

        all_bytes.push(RTMP_VERSION);
        all_bytes.extend(s1);
        all_bytes.extend(s2);
    }

    Ok(all_bytes)
}

/// Generates RTMP handshake response (S1)
/// msg_format - Message format
/// logger - Logger of the RTMP session
pub fn generate_s1(msg_format: u32, logger: &Logger) -> Result<Vec<u8>, ()> {
    let mut random_bytes = vec![0; RTMP_SIG_SIZE - 8];

    let mut rng = StdRng::from_os_rng();

    rng.fill_bytes(&mut random_bytes);

    let mut handshake_bytes: Vec<u8> = vec![0, 0, 0, 0, 1, 2, 3, 4];

    handshake_bytes.extend(random_bytes);

    if handshake_bytes.len() < RTMP_SIG_SIZE {
        handshake_bytes.resize(RTMP_SIG_SIZE, 0);
    } else {
        handshake_bytes.truncate(RTMP_SIG_SIZE);
    }

    let server_digest_offset = if msg_format == MESSAGE_FORMAT_1 {
        get_client_genuine_const_digest_offset(&handshake_bytes[8..12])
    } else {
        get_client_genuine_const_digest_offset(&handshake_bytes[772..776])
    };

    let mut msg: Vec<u8> = vec![0; server_digest_offset];

    if handshake_bytes.len() < server_digest_offset + SHA256DL {
        log_debug!(
            logger,
            format!(
                "Handshake bytes too small. Expected at least {}, but found {}",
                server_digest_offset + SHA256DL,
                handshake_bytes.len()
            )
        );

        return Err(());
    }

    msg.copy_from_slice(&handshake_bytes[0..server_digest_offset]);

    if handshake_bytes.len() > server_digest_offset + SHA256DL {
        msg.extend(&handshake_bytes[server_digest_offset + SHA256DL..]);
    }

    let forced_msg_len = RTMP_SIG_SIZE - SHA256DL;

    if msg.len() < forced_msg_len {
        msg.resize(forced_msg_len, 0);
    } else {
        msg.truncate(forced_msg_len);
    }

    let h = calc_hmac(&msg, GENUINE_FMS.as_bytes());

    if h.len() != SHA256DL {
        log_debug!(
            logger,
            format!(
                "HMAC size invalid. Expected {}, but found {}",
                SHA256DL,
                h.len()
            )
        );

        return Err(());
    }

    handshake_bytes[server_digest_offset..server_digest_offset + SHA256DL].copy_from_slice(&h);

    Ok(handshake_bytes)
}

/// Generates RTMP handshake response (S2)
/// msg_format - Message format
/// client_signature - Client signature
/// logger - Logger of the RTMP session
pub fn generate_s2(
    msg_format: u32,
    client_signature: &[u8],
    logger: &Logger,
) -> Result<Vec<u8>, ()> {
    if client_signature.len() < 776 {
        log_debug!(
            logger,
            format!(
                "Client signature is too small. Expected at least 776, but found {}",
                client_signature.len()
            )
        );
        return Err(());
    }

    let mut random_bytes = vec![0; RTMP_SIG_SIZE - 32];

    let mut rng = StdRng::from_os_rng();

    rng.fill_bytes(&mut random_bytes);

    let challenge_key_offset = if msg_format == MESSAGE_FORMAT_1 {
        get_client_genuine_const_digest_offset(&client_signature[8..12])
    } else {
        get_server_genuine_const_digest_offset(&client_signature[772..776])
    };

    if client_signature.len() < challenge_key_offset + SHA256K {
        log_debug!(
            logger,
            format!(
                "Client signature is too small. Expected at least {}, but found {}",
                challenge_key_offset + SHA256K,
                client_signature.len()
            )
        );
        return Err(());
    }

    let challenge_key = &client_signature[challenge_key_offset..challenge_key_offset + SHA256K];

    let h = calc_hmac(challenge_key, &GENUINE_FMS_PLUS_CRUD);
    let signature = calc_hmac(&random_bytes, &h);

    let mut s2_bytes: Vec<u8> = vec![0; random_bytes.len() + signature.len()];

    s2_bytes[0..random_bytes.len()].copy_from_slice(&random_bytes);
    s2_bytes[random_bytes.len()..].copy_from_slice(&signature);

    if s2_bytes.len() < RTMP_SIG_SIZE {
        s2_bytes.resize(RTMP_SIG_SIZE, 0);
    } else {
        s2_bytes.truncate(RTMP_SIG_SIZE);
    }

    Ok(s2_bytes)
}

/// Calculates HMAC
fn calc_hmac(message: &[u8], key: &[u8]) -> Vec<u8> {
    let mut mac: Hmac<Sha256> = Hmac::new_from_slice(key).expect("HMAC can take key of any size");

    mac.update(message);

    let result: Vec<u8> = mac.finalize().into_bytes().iter().copied().collect();

    result
}

/// Compares 2 signatures
/// Returns true only if the 2 signatures are equal
fn compare_signatures(sig1: &[u8], sig2: &[u8]) -> bool {
    if sig1.len() != sig2.len() {
        return false;
    }

    let mut result = true;

    for i in 0..sig1.len() {
        result = result && (sig1[i] == sig2[i]);
    }

    result
}

/// Detects message format from client signature
fn detect_client_message_format(client_signature: &[u8], logger: &Logger) -> Result<u32, ()> {
    if client_signature.len() < 776 {
        log_debug!(
            logger,
            format!(
                "Client signature is too small. Expected at least 776, but found {}",
                client_signature.len()
            )
        );
        return Err(());
    }

    {
        let sdl = get_server_genuine_const_digest_offset(&client_signature[772..776]);

        let mut msg = vec![0; sdl];

        if client_signature.len() < sdl + SHA256DL {
            log_debug!(
                logger,
                format!(
                    "Client signature is too small. Expected at least {}, but found {}",
                    sdl + SHA256DL,
                    client_signature.len()
                )
            );

            return Err(());
        }

        msg.copy_from_slice(&client_signature[0..sdl]);

        if client_signature.len() > sdl + SHA256DL {
            msg.extend(&client_signature[sdl + SHA256DL..]);
        }

        if msg.len() < 1504 {
            msg.resize(1504, 0);
        } else {
            msg.truncate(1504);
        }

        let computed_signature = calc_hmac(&msg, GENUINE_FP.as_bytes());
        let provided_signature = &client_signature[sdl..sdl + SHA256DL];

        if compare_signatures(&computed_signature, provided_signature) {
            return Ok(MESSAGE_FORMAT_2);
        }
    }

    {
        let sdl_2 = get_client_genuine_const_digest_offset(&client_signature[8..12]);
        let mut msg2 = vec![0; sdl_2];

        if client_signature.len() < sdl_2 + SHA256DL {
            log_debug!(
                logger,
                format!(
                    "Client signature is too small. Expected at least {}, but found {}",
                    sdl_2 + SHA256DL,
                    client_signature.len()
                )
            );

            return Err(());
        }

        msg2.copy_from_slice(&client_signature[0..sdl_2]);

        if client_signature.len() > sdl_2 + SHA256DL {
            msg2.extend(&client_signature[sdl_2 + SHA256DL..]);
        }

        if msg2.len() < 1504 {
            msg2.resize(1504, 0);
        } else {
            msg2.truncate(1504);
        }

        let computed_signature = calc_hmac(&msg2, GENUINE_FP.as_bytes());
        let provided_signature = &client_signature[sdl_2..sdl_2 + SHA256DL];

        if compare_signatures(&computed_signature, provided_signature) {
            return Ok(MESSAGE_FORMAT_1);
        }
    }

    Ok(MESSAGE_FORMAT_0)
}

/// Gets the basic digest of the RTMP Genuine const of the client
fn get_client_genuine_const_digest_offset(buf: &[u8]) -> usize {
    if buf.len() < 4 {
        return 0;
    }

    (((buf[0] as usize) + (buf[1] as usize) + (buf[2] as usize) + (buf[3] as usize)) % 728) + 12
}

/// Gets the basic digest of the RTMP Genuine const of the server
fn get_server_genuine_const_digest_offset(buf: &[u8]) -> usize {
    if buf.len() < 4 {
        return 0;
    }

    (((buf[0] as usize) + (buf[1] as usize) + (buf[2] as usize) + (buf[3] as usize)) % 728) + 776
}
