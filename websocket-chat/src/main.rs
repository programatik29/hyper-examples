mod actor;
mod room;

use actor::{start_actor, SenderHandle, ReceiverHandle};
use room::{ChatRoom, RoomReceiver};

use std::include_bytes;
use std::convert::Infallible;

use tokio::net::TcpListener;

use hyper::{Request, Response, Body};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;

use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::Role;

use sha1::{Digest, Sha1};

type AnyError = Box<dyn std::error::Error + Send + Sync>;

pub static INDEX: &'static [u8] = include_bytes!("../html/index.html");

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let room = ChatRoom::new();
    let room: &'static ChatRoom = Box::leak(Box::new(room));

    let listener = TcpListener::bind("127.0.0.1:3000").await?;

    loop {
        let (stream, _addr) = listener.accept().await?;

        tokio::spawn(async move {
            let fut = Http::new()
                .serve_connection(stream, service_fn(|req| serve(req, room)))
                .with_upgrades();

            match fut.await {
                Ok(()) => (),
                Err(_) => (),
            }
        });
    }
}

async fn serve(
    req: Request<Body>,
    room: &'static ChatRoom
) -> Result<Response<Body>, Infallible> {
    match try_serve(req, room).await {
        Ok(resp) => Ok(resp),
        Err(_) => Ok(not_found()),
    }
}


fn not_found() -> Response<Body> {
    Response::builder()
        .status(404)
        .body(Body::from("Not Found"))
        .unwrap()
}

async fn try_serve(
    req: Request<Body>,
    room: &'static ChatRoom
) -> Result<Response<Body>, AnyError> {
    match req.uri().path() {
        "/" => {
            let resp = Response::builder()
                .body(Body::from(INDEX))
                .unwrap();

            Ok(resp)
        },
        "/websocket" => {
            Ok(
                upgrade(req, room.clone()).await?
            )
        },
        _ => Ok(not_found())
    }
}

async fn upgrade(
    mut req: Request<Body>,
    room: ChatRoom,
) -> Result<Response<Body>, AnyError> {
    let key = req.headers()
        .get("sec-websocket-key").ok_or("")?
        .to_str()?
        .to_owned();

    tokio::spawn(async move {
        match hyper::upgrade::on(&mut req).await {
            Ok(upgraded) => {  
                handle_websocket(upgraded, room).await;
            }
            Err(e) => println!("error upgrading connection: {:?}", e),
        }
    });

    let resp = Response::builder()
        .status(101)
        .header("connection", "Upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-accept", convert_key(key.as_bytes()))
        .body(Body::empty()).unwrap();

    Ok(resp)
}

fn convert_key(input: &[u8]) -> String {
    const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::default();
    sha1.update(input);
    sha1.update(WS_GUID);
    base64::encode(&sha1.finalize())
}

async fn handle_websocket(
    upgrade: Upgraded,
    room: ChatRoom
) {
    let ws_stream = WebSocketStream::from_raw_socket(
        upgrade,
        Role::Server,
        None
    ).await;

    let (tx, mut rx) = start_actor(ws_stream);

    let username = loop {
        if let Some(res) = rx.recv().await {
            if let Ok(Message::Text(username)) = res {
                break username;
            }
        } else {
            return;
        }
    };

    let room_receiver = match room.join(username.clone()) {
        Some(r) => r,
        None => {
            let _ = tx.send(Message::Text("username already taken".to_owned())).await;
            return;
        }
    };

    let _ = room.send(Message::Text(format!("{} joined the room", username)));

    let handle = tokio::spawn(send_task(room_receiver, tx));
    receive_task(room, username, rx).await;

    handle.abort();
}

async fn send_task(mut room_receiver: RoomReceiver, tx: SenderHandle) {
    while let Ok(msg) = room_receiver.recv().await {
        if let Err(_) = tx.send(msg).await {
            break;
        }
    }
}

async fn receive_task(room: ChatRoom, username: String, mut rx: ReceiverHandle) {
    while let Some(res) = rx.recv().await {
        if let Ok(Message::Text(s)) = res {
            let msg = Message::Text(format!("{}: {}", username, s));

            room.send(msg);
        }
    }

    let msg = Message::Text(format!("{} left the room", username));

    room.send(msg);
}
