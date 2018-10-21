extern crate env_logger;
extern crate erpc;

fn main() {
    env_logger::init();
    erpc::run();
}
