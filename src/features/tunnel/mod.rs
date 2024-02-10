use std::{thread, time::Duration};
extern crate chrono;
use actix_web::{App, HttpServer, middleware, web};
use tokio::time as tokio_time;
use tokio::runtime::Runtime;
mod app_state;
mod client;
mod server;

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    let tick_timer = timer::Timer::new();
    let state = web::Data::new(app_state::AppState::new());
    let new_state = state.clone();
    let _guard = tick_timer.schedule_repeating(chrono::Duration::seconds(5), move || {
        new_state.flush_client();
    });
    HttpServer::new(move || {
        let app = App::new()
            .app_data(state.clone()).wrap(middleware::Logger::default());
        app.service(server::scope("/tunnel/v1/server"))
        .service(client::scope("/tunnel/v1/client"))
        .route("/", web::get().to(||async{ "Welcome To Use File Tunnel" }))
    }).bind(("0.0.0.0", 8809))?
    .run().await
}

