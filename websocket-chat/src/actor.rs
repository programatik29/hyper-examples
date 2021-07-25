use futures::stream::StreamExt;
use futures::sink::SinkExt;

use tokio::sync::{mpsc, oneshot};
use tokio::io::{AsyncRead, AsyncWrite};

use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::{
    Message,
    error::Error as WsError,
    error::Result as WsResult
};

type ActorSendData = WsResult<Message>;
type ActorReceiveData = (Message, oneshot::Sender<WsError>);

type ActorSender = mpsc::Sender<ActorSendData>;
type ActorReceiver = mpsc::Receiver<ActorReceiveData>;

type UserSender = mpsc::Sender<ActorReceiveData>;
type UserReceiver = mpsc::Receiver<ActorSendData>;

pub struct WebsocketActor<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static
{
    ws: WebSocketStream<S>,
    rx: ActorReceiver,
    tx: ActorSender
}

impl<S> WebsocketActor<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static
{
    async fn run(mut self) {
        loop {
            tokio::select! {
                opt = self.ws.next() => if let Err(_) = self.recv_msg(opt).await {
                    break;
                },
                opt = self.rx.recv() => if let Err(_) = self.send_msg(opt).await {
                    break;
                }
            }
        }
    }

    async fn send_msg(&mut self, opt: Option<ActorReceiveData>) -> Result<(), ()> {
        match opt {
            Some((msg, sender)) => if let Err(e) = self.ws.send(msg).await {
                let _ = sender.send(e);

                return Err(());
            }
            None => return Err(())
        }

        Ok(())
    }

    async fn recv_msg(&mut self, opt: Option<ActorSendData>) -> Result<(), ()> {
        let ws_msg = match opt {
            Some(v) => v,
            _ => return Err(())
        };

        if let Err(_) = self.tx.send(ws_msg).await {
            return Err(());
        }

        Ok(())
    }
}

pub struct SenderHandle {
    tx: UserSender
}

impl SenderHandle {
    pub async fn send(&self, msg: Message) -> WsResult<()> {
        let (tx, rx) = oneshot::channel();

        if let Err(_) = self.tx.send((msg, tx)).await {
            return Err(WsError::AlreadyClosed);
        }

        if let Ok(e) = rx.await {
            return Err(e);
        }

        Ok(())
    }
}

pub struct ReceiverHandle {
    rx: UserReceiver
}

impl ReceiverHandle {
    pub async fn recv(&mut self) -> Option<WsResult<Message>> {
        self.rx.recv().await
    }
}

pub fn start_actor<S>(ws: WebSocketStream<S>) -> (SenderHandle, ReceiverHandle)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static
{
    let (actor_tx, user_rx) = mpsc::channel(8);
    let (user_tx, actor_rx) = mpsc::channel(8);

    let actor = WebsocketActor {
        ws,
        tx: actor_tx,
        rx: actor_rx
    };

    tokio::spawn(actor.run());

    (SenderHandle { tx: user_tx }, ReceiverHandle { rx: user_rx })
}
