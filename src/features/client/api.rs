use std::{io::Read as _, time::Duration};
use crate::{
    common::{config::{self, Config}, CommomResult},
    features::commands::{CommandMessage, ApiCommand}
};


pub fn do_http_request_data(cli_config: &mut Config, cmd: &ApiCommand) -> CommomResult<CommandMessage> {
    let chars = do_http_request_raw(cli_config, cmd)?;
    Ok(serde_json::from_slice(chars.as_slice())?)
}

pub fn do_http_request(cli_config: &mut Config, cmd: &ApiCommand) -> CommomResult<Vec<char>> {
    let body = do_http_request_raw(cli_config, cmd)?;
    let chars: Vec<char> = body.iter().map(|u| *u as char).collect();
    Ok(chars)
}

pub fn do_http_request_raw(cli_config: &mut Config, cmd: &ApiCommand) -> CommomResult<Vec<u8>> {
    let config_map = cli_config.get_keys_to_map(Some(vec![
        config::CFG_SHARE_KEY.to_string(),
        config::CFG_CLIENT_KEY.to_string(),
        config::CFG_TUNNEL_HOST.to_string(),
    ]));
    let share_key = config_map.get(config::CFG_SHARE_KEY).unwrap();
    let client_key = config_map.get(config::CFG_CLIENT_KEY).unwrap();
    let tunnel_host = config_map.get(config::CFG_TUNNEL_HOST).unwrap();

    let http_cli = reqwest::blocking::Client::new();
    let body = serde_json::to_string(&cmd).unwrap();
    // println!("req: {}", &body[..100]);
    let url_endpoint = format!("http://{}/{}", tunnel_host, "tunnel/v1/client/data");
    
    let mut res = http_cli.post(url_endpoint.clone())
                .timeout(Duration::from_secs(120))
                .header("X-Server-Key", share_key)
                .header("X-Client-Key", client_key)
                .body(body).send()?;
    
    let mut body: Vec<u8> = vec![];
    loop {
        let mut buffer: Vec<u8> = vec![0u8; 32];
        let _usize = res.read(&mut  buffer)?;
        if _usize == 0 {
            break ;
        }
        
        body.extend(buffer[.._usize].iter());
    }
    // println!("res: {}", &String::from_utf8(body.clone()).unwrap()[..100]);
    Ok(body)
}