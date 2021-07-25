mod rustls_config;

use std::include_bytes;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use futures::sink::SinkExt;
use futures::stream::StreamExt;

use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

use std::convert::Infallible;

use hyper::{Body, Request, Response};
use hyper::service::service_fn;
use hyper::server::conn::Http;
use hyper::upgrade::Upgraded;

use sha1::{Digest, Sha1};

const INDEX: &'static [u8] = include_bytes!("../html/index.html");

pub type AnyError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let server_config = Arc::new(
        rustls_config::server_config("certs/key.pem", "certs/cert.pem")?
    );

    let acceptor = TlsAcceptor::from(server_config);
    let listener = TcpListener::bind("127.0.0.1:3443").await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            if let Ok(stream) = acceptor.accept(stream).await {
                let fut = Http::new()
                    .serve_connection(stream, service_fn(serve))
                    .with_upgrades();

                match fut.await {
                    Ok(()) => (),
                    Err(_) => (),
                }
            }

        });
    }
}

async fn serve(
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    match try_serve(req).await {
        Ok(resp) => Ok(resp),
        Err(_) => Ok(default_error()),
    }
}

async fn try_serve(
    mut req: Request<Body>,
) -> Result<Response<Body>, AnyError> {
    let path = req.uri().path();

    if path == "/websocket" {
        let key = req.headers().get("sec-websocket-key").ok_or("")?.to_str()?.to_owned();

        tokio::task::spawn(async move {
            match hyper::upgrade::on(&mut req).await {
                Ok(upgraded) => {
                    if let Err(e) = handle_connection(upgraded).await {
                        println!("error handling websocket: {}", e)
                    };
                }
                Err(e) => println!("upgrade error: {}", e),
            }
        });

        let resp = Response::builder()
            .status(101)
            .header("connection", "Upgrade")
            .header("upgrade", "websocket")
            .header("sec-websocket-accept", convert_key(key.as_bytes()))
            .body(Body::empty())
            .unwrap();

        Ok(resp)
    } else {
        let resp = Response::builder()
            .status(200)
            .header("content-type", "text/html")
            .body(Body::from(INDEX))
            .unwrap();

        Ok(resp)
    }
}

async fn handle_connection(stream: Upgraded) -> Result<(), AnyError> {
    let mut ws_stream = WebSocketStream::from_raw_socket(
        stream,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None
    ).await;

    while let Some(msg_res) = ws_stream.next().await {
        if let Message::Text(s) = msg_res? {
            let msg = Message::Text(s);

            ws_stream.send(msg).await?;
        }
    }

    Ok(())
}

fn convert_key(input: &[u8]) -> String {
    const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::default();
    sha1.update(input);
    sha1.update(WS_GUID);
    base64::encode(&sha1.finalize())
}

fn default_error() -> Response<Body> {
    Response::builder()
        .status(500)
        .header("content-type", "text/plain")
        .body(Body::from("Some Error"))
        .unwrap()
}
