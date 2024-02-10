use common::features;
use env_logger::Env;
fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("error"));
    features::client::main();
}