pub type AnyError = Box<dyn std::error::Error + Send + Sync>;

mod util;
use util::Domain;

use std::sync::Arc;
use std::convert::Infallible;

use tokio::net::TcpListener;

use tokio_rustls::TlsAcceptor;

use hyper::{Body, Request, Response};
use hyper::server::conn::Http;
use hyper::service::service_fn;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let domains: Vec<Domain> = serde_json::from_reader(
        std::fs::File::open("config.json")?
    )?;

    let server_config = util::server_config_from(domains)?;
    let server_config = Arc::new(server_config);

    let listener = TcpListener::bind("127.0.0.1:3443").await?;
    let acceptor = TlsAcceptor::from(server_config);

    loop {
        let (stream, _) = listener.accept().await?;
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            let stream = acceptor.accept(stream).await?;
            let sni = stream.get_ref().1.get_sni_hostname().ok_or("")?.to_owned();

            let fut = Http::new()
                .serve_connection(stream, service_fn(move |req| {
                    let sni = sni.clone();

                    serve(req, sni)
                }));

            match fut.await {
                Ok(()) => (),
                Err(_) => (),
            }

            Ok::<(), AnyError>(())
        });
    }
}

async fn serve(
    _req: Request<Body>,
    sni: String,
) -> Result<Response<Body>, Infallible> {
    let s = format!("SNI is: {}", sni);

    let resp = Response::builder()
        .header("content-type", "text/plain; charset=utf-8")
        .body(Body::from(s))
        .unwrap();

    Ok(resp)
}
