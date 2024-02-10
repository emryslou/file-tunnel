use std::{collections::HashMap, io::{ErrorKind, Read as _}, str::FromStr, time::Duration};
use reqwest::{blocking::Client, header::{HeaderMap, HeaderName, HeaderValue}, StatusCode};
use tide::http::url;

use crate::{
    common::{common_err, config::{self, Config}, CommomResult},
    features::commands::{ApiCommand, CommandMessage}
};

pub struct RequestClient {
    pub http: Client,
    pub host: String,
    headers: HeaderMap,
}

impl  RequestClient  {
    pub fn new(host: &str) -> Self {
        Self { http: Client::new(), host: host.to_string(), headers: HeaderMap::new() }
    }
}

impl  RequestClient  {
    pub fn add_header(&mut self, key: &str, value: &str) -> CommomResult<()> {
        self.headers.insert(
            HeaderName::from_str(key)?, 
            HeaderValue::from_str(value)?
        );
        Ok(())
    }

    pub fn clear_header(&mut self) {
        self.headers.clear();
    }

    pub fn headers(&self) -> HeaderMap {
        self.headers.clone()
    }
}


pub fn do_http_request_data(cli: &RequestClient, cmd: &ApiCommand) -> CommomResult<CommandMessage> {
    let chars = do_http_request_raw(cli, cmd)?;
    Ok(serde_json::from_slice(chars.as_slice())?)
}

pub fn do_http_request(http_cli: &RequestClient, cmd: &ApiCommand) -> CommomResult<Vec<char>> {
    let body = do_http_request_raw(http_cli, cmd)?;
    let chars: Vec<char> = body.iter().map(|u| *u as char).collect();
    Ok(chars)
}

pub fn do_http_request_raw(http_cli: &RequestClient, cmd: &ApiCommand) -> CommomResult<Vec<u8>> {
    let body = serde_json::to_string(&cmd).unwrap();
    let url_endpoint = format!("http://{}/{}", http_cli.host, "tunnel/v1/client/data");
    log::debug!("endpoint: {} {:?}", url_endpoint, cmd);
    let mut res = http_cli.http.post(url_endpoint.clone())
                .timeout(Duration::from_secs(120))
                .headers(http_cli.headers())
                .body(body).send()?;
    match res.status() {
        StatusCode::OK|StatusCode::GATEWAY_TIMEOUT => {
            let mut body: Vec<u8> = vec![];
            loop {
                let mut buffer: Vec<u8> = vec![0u8; 32];
                let _usize = res.read(&mut  buffer)?;
                if _usize == 0 {
                    break ;
                }
                body.extend(buffer[.._usize].iter());
            }
            log::debug!("res: {}", &String::from_utf8(body.clone())?);
            Ok(body)
        },
        state => {
            Err(common_err(format!("request error, status: {}", state).as_str(), None))
        }
    }
    
}

pub fn make_http_client(config: &mut Config) -> CommomResult<RequestClient> {
    let config_map = config.get_keys_to_map(Some(vec![
        config::CFG_SHARE_KEY.to_string(),
        config::CFG_CLIENT_KEY.to_string(),
        config::CFG_TUNNEL_HOST.to_string(),
    ]));
    let share_key = config_map.get(config::CFG_SHARE_KEY).unwrap();
    let client_key = config_map.get(config::CFG_CLIENT_KEY).unwrap();
    let tunnel_host = config_map.get(config::CFG_TUNNEL_HOST).unwrap();
    let mut rc = RequestClient::new(&tunnel_host);

    rc.add_header("X-Server-Key", &share_key)?;
    rc.add_header("X-Client-Key", &client_key)?;
    rc.add_header("Content-Type", "application/json")?;

    Ok(rc)
}
