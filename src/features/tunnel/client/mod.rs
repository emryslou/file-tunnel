use tokio::time::{timeout_at, Instant};
use tide::Request;
use std::time::Duration;

use crate::features::commands::{CommandData, CommandMessage};

use super::websocket_channel;

pub fn binding(app: &mut tide::Server<()>) {
    app.at("/client").nest({
        let mut client = tide::new();
        client.at("/data").post(receive_data);
        client
    });
}

async fn receive_data(mut req: Request<()>) -> tide::Result {
    let server_key = if let Some(server_keys) = req.header("X-Server-Key") {
         server_keys.get(0).unwrap().to_string()
    } else { "".to_string() };
    let client_key = if let Some(client_keys) = req.header("X-Client-Key") {
         client_keys.get(0).unwrap().to_string()
    } else { "".to_string() };
    
    let mut res = tide::Response::new(401);
    match (server_key.as_str(), client_key.as_str()) {
        ("", "") | ("", _) | (_, "") => res.set_body("server or client key required"),
        keys => {
            let ws_cmd = req.body_string().await.unwrap();
            if let Some(_) = websocket_channel::get(keys.0).await {
                websocket_channel::websocket_send_text(keys.0, keys.1, ws_cmd).await;
                websocket_channel::proxy_open(keys.0, keys.1).await;
                if let Err(_)  = timeout_at(
                    Instant::now() + Duration::from_secs(60), 
                    recv_loop(&mut res, keys.0, keys.1)
                ).await {
                    res.set_status(502);
                        let data = CommandMessage {
                        version: 1,
                        status: 403,
                        data: CommandData::Error { message: "receving data from server time out".to_string()},
                    };
                    res.set_body(serde_json::to_string(&data).unwrap());    
                }
            } else {
                res.set_status(403);
                let data = CommandMessage {
                    version: 1,
                    status: 403,
                    data: CommandData::Error { message: "share key may be off line".to_string()},
                };
                res.set_body(serde_json::to_string(&data).unwrap());
            }
        }
    }
    
    Ok(res)
}

async fn recv_loop(res: &mut tide::Response, server_key: &str, client_key: &str) {
    let mut body: Vec<char> = vec![];
    if let Some(receiver) = websocket_channel::proxy_receive(server_key, client_key).await {
        let postfix = [0u8; 4];
        let mut status = 200;
        loop {
            match receiver.recv().await {
                Ok(data) => {
                    let chars: Vec<char> = data.iter().map(|b| *b as char).collect();
                    if data.ends_with(&postfix) {
                        body.extend(chars[..(chars.len() - postfix.len())].iter());
                        break ;
                    } else {
                        body.extend(chars.iter());
                    }
                },
                Err(_e) => {
                    status = 500;
                    let msg = "receive msg failed";
                    body.extend(msg.chars());
                    eprintln!("{}{}", msg, _e.to_string());
                    break ;
                }
            }
            
        }
        websocket_channel::proxy_close(server_key, client_key).await;
        res.set_status(status);
        println!("body: {:?}", String::from_iter(body[..50].iter()));
        
    } else {
        res.set_status(403);
    }
    res.set_body(String::from_iter(body));
}