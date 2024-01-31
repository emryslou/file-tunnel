#![allow(dead_code)]

use async_std::{channel::{self, Receiver, Sender}, stream::StreamExt, sync::Mutex};
use lazy_static::lazy_static;
use std::{collections::HashMap, time::{Duration, SystemTime}};

use tide_websockets::WebSocketConnection;
use crate::common::CommomResult;

type ShareKey = String;
type RequestSender = Sender<Vec<u8>>;
type RequestReceiver = Receiver<Vec<u8>>;
type RequestChannel = (RequestSender, RequestReceiver);

struct WebSocketChannel {
    ws_conn: WebSocketConnection,
    proxy: HashMap<ShareKey, RequestChannel>,
    proxy_keys: HashMap<ShareKey, (SystemTime, Duration)>
}

impl Drop for WebSocketChannel {
    fn drop(&mut self) {
        self.proxy.clear();
        self.proxy_keys.clear();
        println!("web socket channel try drop");
    }
}

pub enum WSChannelSendType {
    Byte(Vec<u8>, Vec<u8>),
    String(String, String)
}

impl WebSocketChannel {
    pub fn proxy_receive(&self, client_key: &str) -> Option<&Receiver<Vec<u8>>> {
        match self.proxy.get(client_key) {
            None => None,
            Some(proxy) => Some(&proxy.1)
        }
    }

    pub async fn proxy_send(&mut self, client_key: &str, data: Vec<u8>) -> CommomResult<()> {
        if let Some(proxy) = self.proxy.get(client_key) {
            proxy.0.send(data).await?;
            let key = self.proxy_keys
                .entry(client_key.to_owned())
                .or_insert((SystemTime::now(), Duration::from_secs(60)));
            (*key).0 = SystemTime::now();
        } else {
            eprintln!("send msg failed {:?}", client_key);
        }
        Ok(())
    }

    pub async fn proxy_add(&mut self, client_key: &str) {
        if !self.proxy.contains_key(client_key) {
            self.proxy.insert(client_key.to_string(), channel::unbounded());
        }
        let key = self.proxy_keys
            .entry(client_key.to_owned())
            .or_insert((SystemTime::now(), Duration::from_secs(10)));
        (*key).0 = SystemTime::now();
    }

    pub fn proxy_keys(&self) -> HashMap<ShareKey, (SystemTime, Duration)> {
        self.proxy_keys.clone()
    }

    pub async fn proxy_del(&mut self, client_key: &str) {
        if self.proxy.contains_key(client_key) {
            self.proxy.remove(client_key);
            self.proxy_keys.remove(client_key);
        }
    }

    pub async fn proxy_status(&self, client_key: &str) -> bool {
        self.proxy.contains_key(client_key)
    }

    pub async fn websocket_send(&self, send_type: WSChannelSendType) {
        match send_type {
            WSChannelSendType::Byte(client_key, data) => {
                let mut msg = vec![client_key.len() as u8];
                msg.extend(client_key);
                msg.extend(data);
                self.ws_conn.send_bytes(msg).await.unwrap()
            },
            WSChannelSendType::String(client_key, s) => {
                let msg = format!("{}:{}{}", client_key.len(), client_key, s);
                self.ws_conn.send_string(msg).await.unwrap()
            },
        }
    }
}

struct WebSocketChannelPool {
    inner: HashMap<String, WebSocketChannel>,
}

impl WebSocketChannelPool {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new()
        }
    }
}

impl WebSocketChannelPool {
    pub fn add(&mut self, key: &str, conn: WebSocketConnection) {
        self.inner.insert(key.to_string(), WebSocketChannel{
            ws_conn: conn, proxy: HashMap::new(), proxy_keys: HashMap::new()
        });
    }

    pub fn get(&self, key: &str) -> Option<&WebSocketChannel> {
        self.inner.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut WebSocketChannel> {
        self.inner.get_mut(key)
    }

    pub fn del(&mut self, key: &str) {
        let _ = self.inner.remove(key).unwrap();
    }

    pub fn proxy_receiver(&self, server_key: &str, client_key: &str) -> Option<&Receiver<Vec<u8>>> {
        match self.get(server_key) {
            Some(ws_channel) => ws_channel.proxy_receive(client_key),
            None => None,
        }
    }

    pub async fn proxy_send(&mut self, server_key: &str, client_key: &str, data: Vec<u8>) -> CommomResult<()> {
        match self.get_mut(server_key) {
            Some(ws_channel) => ws_channel.proxy_send(client_key, data).await,
            None => Ok(()),
        }
    }

    pub async fn proxy_open(&mut self, server_key: &str, client_key: &str) {
        match self.get_mut(server_key) {
            Some(ws_channel) => {
                ws_channel.proxy_add(client_key).await;
            },
            None => {
                eprintln!("open proxy failed");
            },
        }
    }

    pub async fn proxy_close(&mut self, server_key: &str, client_key: &str) {
        if self.proxy_status(server_key, client_key).await {
            self.get_mut(server_key).unwrap().proxy_del(client_key).await;
        }
    }

    pub async fn proxy_status(&self, server_key: &str, client_key: &str) -> bool {
        match self.get(server_key) {
            Some(ws_channel) => ws_channel.proxy_status(client_key).await,
            None => false
        }
    }
}

lazy_static! {
    static ref WS_CHANNEL: Mutex<Box<WebSocketChannelPool>> = Mutex::new(Box::new(WebSocketChannelPool::new()));
}

pub async fn add(server_key: &str, conn: WebSocketConnection) {
    WS_CHANNEL.lock().await.add(server_key, conn);
}

pub async fn get(server_key: &str) -> Option<WebSocketConnection> {
    match WS_CHANNEL.lock().await.get(server_key) {
        Some(ws_chann) => Some(ws_chann.ws_conn.clone()),
        None => None,
    }
}

pub async fn del(server_key: &str) {
    WS_CHANNEL.lock().await.del(server_key);
}

pub async fn proxy_open(server_key: &str, client_key: &str) {
    WS_CHANNEL.lock().await.proxy_open(server_key, client_key).await;
}

pub async fn proxy_close(server_key: &str, client_key: &str) {
    WS_CHANNEL.lock().await.proxy_close(server_key, client_key).await;
}

pub async fn proxy_close_expired() {
    eprintln!("todo");
}

pub async fn proxy_receive<'a, 'b>(server_key: &'a str, client_key: &'a str) -> Option<Receiver<Vec<u8>>> {
    match WS_CHANNEL.lock().await.proxy_receiver(server_key, client_key) {
        None => None,
        Some(r) => Some(r.clone())
    }
}

pub async fn proxy_send(server_key: &str, client_key: &str, message: Vec<u8>) -> CommomResult<()> {
    WS_CHANNEL.lock().await.proxy_send(server_key, client_key, message).await
}

pub async fn websocket_send_bytes(server_key: &str, client_key: &str, message: Vec<u8>) {
    WS_CHANNEL.lock().await
        .get(server_key).unwrap()
        .websocket_send(WSChannelSendType::Byte(client_key.as_bytes().to_vec(), message)).await;
}

pub async fn websocket_send_text(server_key: &str, client_key: &str, message: String) {
    WS_CHANNEL.lock().await
        .get(server_key).unwrap()
        .websocket_send(WSChannelSendType::String(client_key.to_string(), message)).await;
}
