use std::collections::HashMap;

use std::sync::mpsc::{Sender, Receiver, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use actix::Actor;

use crate::common::{common_err, CommomResult};

use super::server::ServerWebSocket;

#[derive(Debug, Clone)]
pub enum ProxyMessage {
    Text(String),
    Binary(Vec<u8>),
}

unsafe impl Send for ProxyMessage {}

type ProxySender = Sender<ProxyMessage>;
type ProxyReceiver = Receiver<ProxyMessage>;

#[derive(Clone)]
pub struct ClientReceiver {
    recv: Arc<Box<ProxyReceiver>>,
    expired_at: SystemTime,
}

unsafe impl Send for ClientReceiver { }

impl ClientReceiver {
    pub fn new(recv: ProxyReceiver) -> Self {
        Self { 
            recv: Arc::new(Box::new(recv)),
            expired_at: SystemTime::now() + Duration::from_secs(10),
        }
    }
    pub fn receiver(&self) -> Arc<Box<ProxyReceiver>> {
        self.recv.clone()
    }

    pub fn delay_expired_at(&mut self, delay: Option<Duration>) {
        match delay {
            Some(delay) => self.expired_at = self.expired_at + delay,
            None => if self.expired_at + Duration::from_secs(9) > SystemTime::now() {
                self.expired_at = self.expired_at + delay.unwrap_or(Duration::from_secs(10))
            }
        }
    }

    pub fn reset_exipred_at(&mut self) {
        self.expired_at = SystemTime::now() + Duration::from_secs(30)
    }

    pub fn expired(&self) -> bool {
        SystemTime::now() > self.expired_at
    }
}

struct ClientReceiverManager {
    inner: HashMap<String, ClientReceiver>,
}

pub struct AppState {
    server_senders: Mutex<HashMap<String, ProxySender>>,
    http_senders: Mutex<HashMap<String, ProxySender>>,
    client_receiver_manager: Mutex<ClientReceiverManager>,
}



impl AppState {
    pub(crate) fn new() -> Self {
        Self {
            server_senders: Mutex::new(HashMap::new()),
            http_senders: Mutex::new(HashMap::new()),
            client_receiver_manager: Mutex::new(ClientReceiverManager{
                inner: HashMap::new(),
            }),
        }
    }
}

impl AppState {
    pub(crate) fn set_server_sender(&self, srv_key: &str, srv_sender: ProxySender) {    
        if let Ok(mut senders) = self.server_senders.lock() {
            if !senders.contains_key(srv_key) {
                senders.insert(srv_key.to_string(), srv_sender);
            }
        }
    }

    pub(crate) fn del_server_sender(&self, srv_key: &str) {
        if let Ok(mut senders) = self.server_senders.lock() {
            if senders.contains_key(srv_key) {
                senders.remove(srv_key);
            }
        }
    }

    pub(crate) fn get_server_sender(&self, srv_key: &str) -> Option<ProxySender>{
        match self.server_senders.lock().unwrap().get(srv_key) {
            Some(s) => Some(s.clone()),
            None => None,
        }
    }

    pub(crate) fn set_client_sender(&self, cli_key: &str, srv_sender: ProxySender) {
        if let Ok(mut senders) = self.http_senders.lock() {
            if !senders.contains_key(cli_key) {
                senders.insert(cli_key.to_string(), srv_sender);
            }
        }
    }

    pub(crate) fn del_client_sender(&self, cli_key: &str) {
        if let Ok(mut senders) = self.http_senders.lock() {
            if senders.contains_key(cli_key) {
                senders.remove(cli_key);
            }
        }
    }

    pub(crate) fn get_client_sender(&self, cli_key: &str) -> Option<ProxySender>{
        match self.http_senders.lock().unwrap().get(cli_key) {
            Some(s) => Some(s.clone()),
            None => None,
        }
    }

    pub(crate) fn server_open(&self, srv_key: &str, srv_sender: ProxySender) {
        log::info!("server open {}", srv_key);
        self.set_server_sender(srv_key, srv_sender);
    }

    pub(crate) fn server_close(&self, srv_key: &str, cli_keys: &Vec<String>) {
        log::info!("server close {}", srv_key);
        self.del_server_sender(srv_key);
        let _ = cli_keys.iter()
                    .map(|cli_key| self.client_close(cli_key))
                    .collect::<Vec<_>>();
    }

    pub(crate) fn client_open(&self, keys: (&str, &str)) {
        if let Ok(mut mgr) = self.client_receiver_manager.lock() {
            match mgr.inner.get_mut(keys.1) {
                Some(rc) => {
                    log::debug!("client {} has been already online, delay expired", keys.1);
                    if rc.expired() {
                        rc.reset_exipred_at();
                    } else {
                        rc.delay_expired_at(Some(Duration::from_secs(5)));
                    }
                },
                None => {
                    log::debug!("client {} is online", keys.1);
                    let (tx, rx) = channel();
                    self.set_client_sender(keys.1, tx);
                    let mut rc = ClientReceiver::new(rx);
                    rc.delay_expired_at(None);
                    mgr.inner.insert(keys.1.to_string(), rc);
                }
            }
        }
    }

    pub fn client_receiver(&self, cli_key: &str) -> Option<Arc<Box<Receiver<ProxyMessage>>>> {
        match self.client_receiver_manager.lock() {
            Err(e) => {
                log::warn!("client_receiver_manager lock failed, err msg: {}", e);
                None
            },
            Ok(crmgr) => Some(crmgr.inner.get(cli_key)?.receiver())
        }
    }

    pub(crate) fn client_close(&self, client_key: &str) {
        log::debug!("client {} offline", client_key);
        self.del_client_sender(client_key);
        self.client_receiver_manager.lock().unwrap().inner.remove(client_key);
    }

    pub fn server_send_text(&self, srv_key: &str, message: &str) -> CommomResult<()> {
        match self.get_server_sender(srv_key) {
            Some(tx) => {
                tx.send(ProxyMessage::Text(message.to_string()))?;
                Ok(())
            },
            None => {
                Err(common_err(format!("server key {:#?} not found", srv_key).as_str(), Some(std::io::ErrorKind::NotFound)))
            },
        }
    }

    pub fn server_send_binary(&self, srv_key: &str, message: Vec<u8>) -> CommomResult<()> {
        match self.get_server_sender(srv_key) {
            Some(tx) => {
                tx.send(ProxyMessage::Binary(message))?;
                Ok(())
            },
            None => {
                Err(common_err(
                    format!("server key {:#?} not found", srv_key).as_str(), 
                    Some(std::io::ErrorKind::NotFound))
                )
            },
        }
    }

    pub fn flush_client(&self) {
        if let Ok(mut clients) = self.client_receiver_manager.lock() {
            let _ = clients.inner.clone().into_iter().map(|(client_key, cr)| {
                if cr.expired() {
                    log::debug!("flush client, client key {} expired and will be removed", client_key);
                    self.del_client_sender(&client_key);
                    clients.inner.remove(&client_key);
                }
            }).collect::<Vec<_>>();
        }
    }
}
