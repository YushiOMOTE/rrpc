use tera::{Context, Tera};
use serde_json::Value;

use crate::utils;
use crate::error::{self, Result};

fn tera(path: &str) -> Tera {
    let mut tera = compile_templates!(path);
    tera.autoescape_on(vec![""]);
    tera
}

pub fn render(path: &str, tpath: &str, model: Value) -> Result<String> {
    let namespace = utils::namespace(path)?;
    let tera = tera(tpath);
    let mut context = Context::new();

    context.insert("ast", &model);
    context.insert("namespace", namespace);

    tera.render("root.cpp", &context)
        .map_err(|e| error::render_error(e))
}
