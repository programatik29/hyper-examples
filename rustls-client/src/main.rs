pub type AnyError = Box<dyn std::error::Error + Send + Sync>;

mod util;

use tokio::net::TcpStream;
use hyper::{Request, Body};
use hyper::client::conn;

use tower::{Service, ServiceExt};

use tokio_rustls::webpki::DNSNameRef;

use rustls::Session;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let connector = util::get_tls_connector()?;

    let stream = TcpStream::connect("tokio.rs:443").await?;

    let domain = DNSNameRef::try_from_ascii_str("tokio.rs")?;
    let stream = connector.connect(domain, stream).await?;

    // Get negotiated alpn protocol.
    let h2_only = match stream.get_ref().1.get_alpn_protocol() {
        Some(b"h2") => true,
        _ => false,
    };

    // Switch to http2 mode if negotiated alpn is http2.
    let (mut sender, connection) = conn::Builder::new()
        .http2_only(h2_only)
        .handshake(stream).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            println!("error: {}", e);
        }
    });

    let req = match h2_only {
        true => get_http2_request(),
        false => get_http1_request()
    };

    // Important when sending consecutive requests.
    // It is a good habit to check every time before sending request.
    sender.ready().await?;
    let res = sender.call(req).await?;

    let (parts, body) = res.into_parts();

    let bytes = hyper::body::to_bytes(body).await.unwrap();

    // Non-utf8 characters are replaced with a replacement character.
    let res_string = String::from_utf8_lossy(&bytes).into_owned();

    println!("Response: {:?}", parts);
    println!("Body: {}", res_string);

    Ok(())
}

fn get_http1_request() -> Request<Body> {
    Request::builder()
        .uri("/")
        .header("Host", "tokio.rs")
        .body(Body::empty())
        .unwrap()
}

fn get_http2_request() -> Request<Body> {
    Request::builder()
        .uri("https://tokio.rs/")
        .body(Body::empty())
        .unwrap()
}
