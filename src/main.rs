#[macro_use]
extern crate log;
extern crate env_logger;
extern crate error_chain;
extern crate rrpc;
extern crate serde_json;

use error_chain::ChainedError;

fn main() {
    env_logger::init();

    let text = match rrpc::compile("null", "examples/init.rpc", "examples/templates/**/*") {
        Ok(text) => text,
        Err(e) => return error!("{}", e.display_chain().to_string()),
    };

    println!("{}", text);
}
