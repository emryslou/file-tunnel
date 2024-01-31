use chrono::Duration;

extern crate timer;

mod websocket_channel;
mod server;
mod client;

#[async_std::main]
pub async fn main() -> tide::Result<()> {
    let mut app = tide::new();
    app.at("/").get(|_| async {
        Ok("Welcome To Use File Tunnel")
    });
    app.at("/tunnel/v1").nest({
        let mut tunnel_v1 = tide::new();
        server::binding(&mut tunnel_v1);
        client::binding(&mut tunnel_v1);
        tunnel_v1
    });
    let new_timer = timer::Timer::new();
    let _guard = new_timer.schedule_repeating(Duration::seconds(10), move || {
        println!("... tick ...");
    });
    app.listen("0.0.0.0:8809").await?;
    Ok(())
}

