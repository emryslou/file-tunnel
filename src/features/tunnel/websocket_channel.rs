#![allow(dead_code)]

use async_std::{sync::Mutex, channel::{self, Receiver, Sender}};
use lazy_static::lazy_static;
use std::collections::HashMap;

use tide_websockets::WebSocketConnection;

type ShareKey = String;

struct WebSocketChannel {
    ws_conn: WebSocketConnection,
    proxy: (Sender<Vec<u8>>, Receiver<Vec<u8>>)
}

pub enum WSChannelSendType {
    Byte(Vec<u8>),
    String(String)
}

impl WebSocketChannel {
    pub fn receiver(&self) -> Option<Receiver<Vec<u8>>> {
        Some(self.proxy.1.clone())
    }

    pub async fn proxy_send(&self, data: Vec<u8>) {
        self.proxy.0.send(data).await.unwrap();
    }

    pub async fn websocket_send(&self, send_type: WSChannelSendType) {
        match send_type {
            WSChannelSendType::Byte(data) => self.ws_conn.send_bytes(data).await.unwrap(),
            WSChannelSendType::String(s) => self.ws_conn.send_string(s).await.unwrap(),
        }
    }
}

struct WebSocketChannelPool {
    inner: HashMap<String, WebSocketChannel>,
    proxy_share_keys: Vec<String>,
}

impl WebSocketChannelPool {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            proxy_share_keys: vec![],
        }
    }
}

impl WebSocketChannelPool {
    pub fn add(&mut self, key: String, conn: WebSocketConnection) {
        let (send, recv) = channel::unbounded();

        self.inner.insert(key, WebSocketChannel{ws_conn: conn, proxy: (send, recv)});
    }

    pub fn get(&self, key: &String) -> Option<&WebSocketChannel> {
        self.inner.get(key)
    }

    pub fn del(&mut self, key: &String) {
        self.inner.remove(key);
    }

    pub fn receiver(&self, key: &String) -> Option<Receiver<Vec<u8>>> {
        match self.get(&key) {
            Some(ws_channel) => ws_channel.receiver(),
            None => None,
        }
    }

    pub async fn send(&mut self, share_key: &String, data: Vec<u8>) {
        match self.get(share_key) {
            Some(ws_channel) => ws_channel.proxy_send(data).await,
            None => {},
        }
    }

    pub async fn proxy_open(&mut self, share_key: &String) {
        self.proxy_share_keys.push(share_key.clone());
    }

    pub async fn proxy_close(&mut self, share_key: &String) {
        if self.proxy_status(&share_key).await {
            self.proxy_share_keys = self.proxy_share_keys.iter()
                        .filter(|psk| *psk == share_key)
                        .map(|psk|psk.clone())
                        .collect();
        }
    }

    pub async fn proxy_status(&self, share_key: &String) -> bool {
        self.proxy_share_keys.contains(share_key)
    }
}

lazy_static! {
    static ref WS_CHANNEL: Mutex<Box<WebSocketChannelPool>> = Mutex::new(Box::new(WebSocketChannelPool::new()));
}

pub async fn add(share_key: String, conn: WebSocketConnection) {
    WS_CHANNEL.lock().await.add(share_key, conn);
}

pub async fn get(share_key: &String) -> Option<WebSocketConnection> {
    match WS_CHANNEL.lock().await.get(share_key) {
        Some(ws_chann) => Some(ws_chann.ws_conn.clone()),
        None => None,
    }
}

pub async fn del(share_key: &String) {
    WS_CHANNEL.lock().await.del(share_key);
}

pub async fn proxy_open(share_key: &String) {
    WS_CHANNEL.lock().await.proxy_open(share_key).await;
}

pub async fn proxy_close(share_key: &String) {
    WS_CHANNEL.lock().await.proxy_close(share_key).await;
}

pub async fn proxy_receive(share_key: &String) -> Option<Receiver<Vec<u8>>> {
    if WS_CHANNEL.lock().await.proxy_status(share_key).await {
        return WS_CHANNEL.lock().await.receiver(share_key);
    }
    None
}

pub async fn proxy_send(share_key: &String, message: Vec<u8>) -> Result<(), channel::SendError<Vec<u8>>> {
    if WS_CHANNEL.lock().await.proxy_status(share_key).await {
        WS_CHANNEL.lock().await.send(share_key, message).await;
    }
    Ok(())
}

pub async fn websocket_send_bytes(share_key: &String, message: Vec<u8>) {
    WS_CHANNEL.lock().await.get(share_key).unwrap().websocket_send(WSChannelSendType::Byte(message)).await;
}

pub async fn websocket_send_text(share_key: &String, message: String) {
    WS_CHANNEL.lock().await.get(share_key).unwrap().websocket_send(WSChannelSendType::String(message)).await;
}
