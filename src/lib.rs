extern crate pest;
#[macro_use]
extern crate pest_derive;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;

mod types;
mod loader;
mod error;
mod resolver;
mod parser;
mod generator;
mod lang;

use serde_json::Value;

use self::error::{Result, ResultExt};
use self::generator::Generator;
use self::lang::NullGenerator;

pub fn compile(path: &str) -> Result<Value> {
    let mut nullgen = NullGenerator;
    let mut gen = Generator::new(&mut nullgen);

    let cwd = std::env::current_dir().map_err(|e| error::file_error(e))?;
    let fullpath = format!("{}/{}", cwd.to_string_lossy(), path);

    let model = gen.generate(path).chain_err(|| error::error(&fullpath))?;

    serde_json::to_value(model).map_err(|e| error::pack_error(e))
}
