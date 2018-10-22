#[macro_use]
extern crate log;
extern crate env_logger;
extern crate error_chain;
extern crate rrpc;
extern crate serde_json;

use error_chain::ChainedError;
use serde_json::to_string_pretty;

fn main() {
    env_logger::init();

    let j = match rrpc::compile("examples/init.rpc") {
        Ok(j) => j,
        Err(e) => return error!("{}", e.display_chain().to_string()),
    };

    info!("{}", to_string_pretty(&j).unwrap_or("".to_string()));
}
