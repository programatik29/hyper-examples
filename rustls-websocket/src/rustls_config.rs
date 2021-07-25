use crate::AnyError;

use std::fs::File;
use std::io::BufReader;

use rustls::{
    ServerConfig,
    NoClientAuth,
    PrivateKey,
};
use rustls::internal::pemfile::{certs, pkcs8_private_keys};

pub fn server_config(
    key_location: &str,
    cert_location: &str,
) -> Result<ServerConfig, AnyError> {
    let mut config = ServerConfig::new(NoClientAuth::new());

    let private_key = get_first_private_key(key_location)?;
    
    let certificates = certs(
        &mut BufReader::new(
            File::open(cert_location)?
        )
    ).map_err(|_| "cant get certificates")?;

    config.set_single_cert(certificates, private_key)
        .map_err(|_| "Invalid certificate chain or private key.")?;

    config.set_protocols(&[b"h2".to_vec(), b"http/1.1".to_vec()]);

    Ok(config)
}

fn get_first_private_key(loc: &str) -> Result<PrivateKey, AnyError> {
    let mut privkeys = pkcs8_private_keys(
        &mut BufReader::new(
            File::open(&loc)?
        )
    ).map_err(|_| "cant get private key")?;

    if !privkeys.is_empty() {
        Ok(privkeys.remove(0))
    } else {
        Err("did not find private key in file".into())
    }
}
