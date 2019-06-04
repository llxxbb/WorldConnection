extern crate dotenv;
extern crate nature;


use std::env;

use actix_web::test::TestServer;
use dotenv::dotenv;

use nature::actor::*;
use nature::channels::start_receive_threads;
use nature::rpc::actix::web_app;
use nature_common::setup_logger;
use actix_web::App;

pub fn test_init() -> TestServer {
    dotenv().ok();

    env::set_var("DATABASE_URL", CONN_STR);

    let _ = setup_logger();

    start_receive_threads();

    TestServer::with_factory(init_all_actor)
}

#[allow(dead_code)]
//pub static CONN_STR : &str = "mysql://root@localhost/nature";
pub static CONN_STR: &str = "nature.sqlite";

fn init_all_actor()->App<()>{
    init_actors();
    web_app()
}