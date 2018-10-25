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
#[macro_use]
extern crate lazy_static;

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

use crate::error::{Result, ResultExt};
use crate::generator::Generator;

pub use crate::types::*;
pub use crate::lang::{register_generator, LangGenerator};
use crate::lang::get_generator;

pub fn compile(gen: &str, path: &str, tpath: &str) -> Result<String> {
    let langgen = get_generator(gen)?;
    let mut langgen = langgen.lock().unwrap();
    let mut gen = Generator::new(&mut *langgen);

    let fullpath = utils::fullpath(path)?;
    let model = gen.generate(path).chain_err(|| error::error(&fullpath))?;
    let model = serde_json::to_value(model).map_err(|e| error::pack_error(e))?;

    debug!(
        "model: {}",
        to_string_pretty(&model).unwrap_or("".to_string())
    );

    render::render(path, tpath, model)
}
