use crate::AnyError;

use rustls::{ResolvesServerCert, ClientHello};

use std::collections::HashMap;

use rustls::sign::CertifiedKey;

// This is only to force rustls to accept 
// localhost certificate for testing purposes
//
// Should be replaced with rustls::ResolvesServerCertUsingSNI
// in production code
pub struct ResolvesServerCertUsingSNI {
    map: HashMap<String, CertifiedKey>,
}

impl ResolvesServerCertUsingSNI {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: &str, ck: CertifiedKey) -> Result<(), AnyError> {
        self.map.insert(name.to_owned(), ck);

        Ok(())
    }
}

impl ResolvesServerCert for ResolvesServerCertUsingSNI {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<CertifiedKey> {
        let sni: String = match client_hello.server_name() {
            // some sort of magic (couldn't find a better way)
            Some(sni) => AsRef::<str>::as_ref(&sni.to_owned()).to_owned(),
            None => return None,
        };

        self.map.get(&sni).cloned()
    }
}
