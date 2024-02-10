use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse, Scope};
use actix_web_actors::ws;
use std::{borrow::Borrow, sync::{mpsc::{channel, Receiver, RecvTimeoutError}, Arc}, thread, time::Duration};
use log;

use crate::common::{common_err, CommomResult};

use super::app_state::{AppState, ProxyMessage};

pub(crate) fn scope(root_path: &str) -> Scope {
    web::scope(root_path)
        .route("/ws", web::get().to(srv_websocket))
}


pub struct ServerWebSocket {
    server_key: String,
    client_keys: Vec<String>,
    state: web::Data<AppState>,
    pub receiver: Arc<Box<Receiver<ProxyMessage>>>
}

impl Actor for ServerWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.receive(ctx);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        self.state.server_close(&self.server_key, &self.client_keys);
        self.client_keys.clear();
    }
}

impl ServerWebSocket{
    pub fn new(server_key: &str, recv: Receiver<ProxyMessage>, state: web::Data<AppState>) -> Self {
        Self { 
            server_key: server_key.to_string(),
            client_keys: vec![],
            receiver: Arc::new(Box::new(recv)),
            state
        }
    }
}

#[allow(dead_code)]
impl ServerWebSocket {
    pub fn client_send_text(&self, client_key: &str, message: &str) -> CommomResult<()> {
        if self.client_keys.contains(&client_key.to_string()) {
            match self.state.get_client_sender(client_key) {
                Some(cli_sender) => {
                    log::info!("send msg {} to client {}", message, client_key);
                    cli_sender.send(ProxyMessage::Text(message.to_string()))?;
                    Ok(())
                },
                None => Err(common_err(format!("client key {:#?} not found", client_key).as_str(), None))
            }
        } else {
            Err(common_err(format!("client key {:#?} may be offline", client_key).as_str(), None))
        }
    }

    pub fn client_send_binary(&self, client_key: &str, message: Vec<u8>) {
        if self.client_keys.contains(&client_key.to_string()) {
            self.state.get_client_sender(client_key).unwrap().send(ProxyMessage::Binary(message)).unwrap();
        } else {
            log::warn!("err: client key {:#?} may be offline ", client_key);
        }
    }

    pub fn add_client_key(&mut self, client_key: &str) {
        if !self.client_keys.contains(&client_key.to_string()) {
            self.client_keys.push(client_key.to_string());
        }
    }
    fn receive(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_millis(99), |me, ctx| {
            match me.receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(message) => {
                    match message {
                        ProxyMessage::Text(text) => {
                            let client_key_size = text[..3].to_string().parse::<usize>().unwrap();
                            let (_, new_text) = text.split_at(3);
                            let (_tmp, _) = new_text.split_at(client_key_size);
                            let client_key = _tmp.to_string();
                            me.add_client_key(client_key.as_str());
                            // let text = text[..245].to_string();
                            log::debug!("recv from {}: {}", client_key, text);
                            ctx.text(text);
                        },
                        ProxyMessage::Binary(_) => todo!(),
                    }
                },
                Err(e) => {
                    match e {
                        RecvTimeoutError::Disconnected => ctx.stop(),
                        RecvTimeoutError::Timeout => (),
                    }
                }
            }
        });
    }
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ServerWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                let (client_key_size_str, msg) = text.split_at(3);
                let client_key_size = match client_key_size_str.parse::<u8>() {
                    Ok(v) => v,
                    Err(_) => 0,
                } as usize;
                if client_key_size > 0 && msg.len() > client_key_size {
                    let client_key = &msg[..client_key_size];
                    let message = msg[client_key_size..].to_string();
                    if let Err(e) = self.client_send_text(client_key, &message) {
                        log::error!("{} {} send fail, echo message, err: {}", client_key, message, e);
                    }
                } else {
                    ctx.text(text);
                }
            },
            Ok(ws::Message::Binary(bin)) => {
                log::info!("binary: {:#?}", bin);
                let client_key_size = bin[0] as usize;
                let client_key_chars = bin[1..(client_key_size + 1)].into_iter().map(|b| *b as char).collect::<Vec<char>>();
                let client_key = String::from_iter(client_key_chars.iter());
                let message = bin[(client_key_size + 1)..].to_vec();
                self.client_send_binary(client_key.as_str(), message);
            },
            Ok(ws::Message::Ping(_)) | Ok(ws::Message::Pong(_)) => (),
            Ok(ws::Message::Continuation(i)) => {
                log::info!("continuation: {:#?}", i);
            },
            Ok(ws::Message::Close(_)) => {
                self.state.server_close(&self.server_key, &self.client_keys);
                self.client_keys.clear();
            },
            Err(e) => { log::error!("some error happends {}", e); },
            s => log::warn!("unknown: {s:#?}")
        }
    }
}

async fn srv_websocket(req: HttpRequest, stream: web::Payload, state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    match req.headers().get("X-Server-Key") {
        Some(server_key) => {
            let (srv_sender, srv_receiver) = channel();
            state.server_open(server_key.to_str().unwrap(), srv_sender.clone());
            let resp = ws::WsResponseBuilder::new(ServerWebSocket::new(
                server_key.to_str().unwrap_or(""),
                srv_receiver,
                state.clone()
            ), &req, stream).frame_size(1 << 20).start();
            resp
        },
        None => {
            let err = std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "server key must be required"
            );
            Err(actix_web::Error::from(err))
        }
    }
}