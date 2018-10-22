#[macro_use]
extern crate log;
extern crate env_logger;
extern crate erpc;
extern crate serde_json;

use serde_json::to_string_pretty;

fn main() {
    env_logger::init();

    let j = match erpc::compile("examples/init.rpc") {
        Ok(j) => j,
        Err(e) => return error!("{}", e),
    };

    info!("{}", to_string_pretty(&j).unwrap_or("".to_string()));
}
