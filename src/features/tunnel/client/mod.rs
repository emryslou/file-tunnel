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
    let share_key = if let Some(share_keys) = req.header("X-Share-Key") {
         share_keys.get(0).unwrap().to_string()
    } else { "".to_string() };
    
    let mut res = tide::Response::new(401);
    match share_key.as_str() {
        "" => res.set_body("share key required"),
        _ => {
            let ws_cmd = req.body_string().await.unwrap();
            if let Some(_) = websocket_channel::get(&share_key).await {
                websocket_channel::websocket_send_text(&share_key, ws_cmd).await;
                websocket_channel::proxy_open(&share_key).await;
                if let Err(_)  = timeout_at(
                    Instant::now() + Duration::from_secs(60), 
                    recv_loop(&mut res, &share_key)
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

async fn recv_loop(res: &mut tide::Response, share_key: &String) {
    let mut body: Vec<char> = vec![];
    if let Some(receiver) = websocket_channel::proxy_receive(&share_key).await {
        let postfix = [0u8; 4];
        loop {
            let data = receiver.recv().await.unwrap();
            let chars: Vec<char> = data.iter().map(|b| *b as char).collect();
            if data.ends_with(&postfix) {
                body.extend(chars[..(chars.len() - postfix.len())].iter());
                break ;
            } else {
                body.extend(chars.iter());
            }
        }
        websocket_channel::proxy_close(&share_key).await;

        res.set_status(200);
        println!("body: {:?}", String::from_iter(body[..50].iter()));
        
    } else {
        res.set_status(403);
    }
    res.set_body(String::from_iter(body));
}