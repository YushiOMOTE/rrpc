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
#[macro_use]
extern crate tera;

mod types;
mod loader;
mod error;
mod resolver;
mod parser;
mod generator;
mod lang;
mod render;
mod utils;

use serde_json::to_string_pretty;

use self::error::{Result, ResultExt};
use self::generator::Generator;
use self::lang::NullGenerator;

pub fn compile(path: &str, tpath: &str) -> Result<String> {
    let mut nullgen = NullGenerator;
    let mut gen = Generator::new(&mut nullgen);

    let fullpath = utils::fullpath(path)?;
    let model = gen.generate(path).chain_err(|| error::error(&fullpath))?;
    let model = serde_json::to_value(model).map_err(|e| error::pack_error(e))?;

    debug!(
        "model: {}",
        to_string_pretty(&model).unwrap_or("".to_string())
    );

    render::render(path, tpath, model)
}
