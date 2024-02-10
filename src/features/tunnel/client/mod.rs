use actix_web::{web, Error, HttpRequest, HttpResponse, Scope};
use reqwest::StatusCode;
use tokio::time::{timeout_at, Instant};
use std::time::Duration;

use crate::features::{
    commands::ApiCommand,
    tunnel::app_state::ProxyMessage
};

use super::app_state::AppState;

pub(crate) fn scope(root_path: &str) -> Scope {
    web::scope(root_path)
        .route("/data", web::post().to(cli_data))
}

async fn cli_data(req: HttpRequest, cmd: web::Json<ApiCommand>, state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let server_key: &str = if let Some(server_key) = req.headers().get("X-Server-Key") {
         server_key.to_str().unwrap_or("")
    } else { "" };
    let client_key = if let Some(client_key) = req.headers().get("X-Client-Key") {
         client_key.to_str().unwrap_or("")
    } else { "" };
    
    match (server_key, client_key) {
        ("", "") => {
            let err = std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "server key must be required"
            );
            Err(actix_web::Error::from(err))
        },
        srv_cli_keys => {
            state.client_open(srv_cli_keys);
            let req_msg = format!("{:03}{}{}", srv_cli_keys.1.len(), srv_cli_keys.1, serde_json::to_string(&cmd).unwrap());
            match state.server_send_text(srv_cli_keys.0, &req_msg) {
                Ok(_) => {
                    let mut body = vec![];
                    let mut resp = match timeout_at(Instant::now() + Duration::from_secs(4), async {
                        match state.client_receiver(srv_cli_keys.1) {
                            Some(recver) => {
                                while let Ok(message) = recver.recv() {
                                    let recv_msg = match message {
                                        ProxyMessage::Text(text) => {
                                            text.as_bytes().to_vec()
                                        },
                                        ProxyMessage::Binary(bin) => bin,
                                    };
                                    if recv_msg.len() > 0 {
                                        body.extend(recv_msg.iter());
                                        if body.ends_with(b"\0\0\0\0") {
                                            body = body[..(body.len() - 4)].to_vec();
                                            break ;
                                        }
                                    }
                                }
                                HttpResponse::Ok()
                            },
                            None => {
                                body.extend(format!("client key {} may be offline", srv_cli_keys.1).as_bytes().iter());
                                HttpResponse::NotFound()
                            }
                        } 
                    }).await {
                        Ok(resp) => resp,
                        Err(e) => {
                            body.extend(e.to_string().as_bytes());
                            HttpResponse::GatewayTimeout()
                        }
                    };
                    Ok(resp.body(body))
                },
                Err(e) => {
                    Err(actix_web::Error::from(e))
                },
            }
        }
    }
}
