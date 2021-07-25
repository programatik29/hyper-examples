use std::ops::{Deref, DerefMut};

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use tokio::sync::broadcast;

use tokio_tungstenite::tungstenite::Message;

type UserMap = Arc<Mutex<HashMap<String, ()>>>;

#[derive(Clone)]
pub struct ChatRoom {
    user_map: UserMap,
    tx: broadcast::Sender<Message>
}

impl ChatRoom {
    pub fn new() -> Self {
        Self {
            user_map: Arc::new(Mutex::new(HashMap::new())),
            tx: broadcast::channel(100).0
        }
    }

    pub fn join(&self, username: String) -> Option<RoomReceiver> {
        let mut user_map = self.user_map.lock().unwrap();

        if user_map.contains_key(&username) {
            return None;
        }

        user_map.insert(username.clone(), ());

        drop(user_map);

        Some(RoomReceiver {
            username,
            user_map: self.user_map.clone(),
            rx: self.tx.subscribe()
        })
    }

    pub fn send(&self, msg: Message) {
        let _ = self.tx.send(msg);
    }
}

pub struct RoomReceiver {
    username: String,
    user_map: UserMap,
    rx: broadcast::Receiver<Message>
}

impl Deref for RoomReceiver {
    type Target = broadcast::Receiver<Message>;

    fn deref(&self) -> &Self::Target {
        &self.rx
    }
}

impl DerefMut for RoomReceiver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rx
    }
}

impl Drop for RoomReceiver {
    fn drop(&mut self) {
        self.user_map.lock().unwrap().remove(&self.username);
    }
}
