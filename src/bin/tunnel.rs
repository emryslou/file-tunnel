use common::features;
use env_logger::Env;
use std::env;

fn main() {    
    env_logger::init_from_env(Env::default().default_filter_or("error"));
    features::tunnel::main().unwrap();
}