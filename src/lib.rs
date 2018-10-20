extern crate pest;
#[macro_use]
extern crate pest_derive;

use pest::Parser;

#[derive(Parser)]
#[grammar = "rpc.pest"]
struct RpcParser;

use std::fs::File;
use std::io::prelude::*;

pub fn run() {
    let mut file = File::open("examples/init.rpc").expect("File not found");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Read failed");

    let pairs = RpcParser::parse(Rule::File, &contents).unwrap_or_else(|e| panic!("{}", e));

    println!("{:#?}", pairs);
}
