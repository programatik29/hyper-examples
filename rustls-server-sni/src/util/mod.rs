mod test;
// only for testing
use test::ResolvesServerCertUsingSNI;
// for production:
// rustls::ResolvesServerCertUsingSNI;

use crate::AnyError;

use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use rustls::{
    ServerConfig,
    NoClientAuth,
    PrivateKey,
};
use rustls::internal::pemfile::{certs, pkcs8_private_keys};
use rustls::sign::{RSASigningKey, CertifiedKey, SigningKey};

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Domain {
    name: String,
    key_location: String,
    cert_location: String,
}

pub fn server_config_from(
    domains: Vec<Domain>,
) -> Result<ServerConfig, AnyError> {
    let mut sconfig = ServerConfig::new(
        NoClientAuth::new()
    );

    let mut sni_resolver = ResolvesServerCertUsingSNI::new();

    for domain in domains {
        let privkey = get_first_private_key(&domain.key_location)?;
        let rsa_key = RSASigningKey::new(&privkey)
            .map_err(|_| "cant get rsa key")?;
        
        let cert = certs(
            &mut BufReader::new(
                File::open(&domain.cert_location)?
            )
        ).map_err(|_| "cant get certificates")?;

        let signing_key: Box<dyn SigningKey> = Box::new(rsa_key);

        let certified_key = CertifiedKey::new(cert, Arc::new(signing_key));

        sni_resolver.add(&domain.name, certified_key)?;
    }

    sconfig.cert_resolver = Arc::new(sni_resolver);

    sconfig.set_protocols(&[b"h2".to_vec(), b"http/1.1".to_vec()]);

    Ok(sconfig)
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
