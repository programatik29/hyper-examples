mod util;

use std::convert::Infallible;

use std::sync::Arc;
use tokio::net::TcpListener;

use tokio_rustls::TlsAcceptor;

use hyper::{Body, Request, Response};
use hyper::service::service_fn;
use hyper::server::conn::Http;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let rustls_config = Arc::new(
        util::rustls_server_config("certs/key.pem", "certs/cert.pem")?
    );

    let acceptor = TlsAcceptor::from(rustls_config);
    let listener = TcpListener::bind("127.0.0.1:3443").await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            if let Ok(stream) = acceptor.accept(stream).await {
                let fut = Http::new()
                    .serve_connection(stream, service_fn(serve));

                match fut.await {
                    Ok(()) => (),
                    Err(_) => (),
                }
            }
        });
    }
}

async fn serve(
  _req: Request<Body>
) -> Result<Response<Body>, Infallible> {
    let resp = Response::builder()
        .body(Body::from("Hello, world!")).unwrap();

    Ok(resp)
}
