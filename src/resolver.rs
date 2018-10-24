use std::collections::HashMap;

use pest::iterators::Pair;

use super::parser::{get, get_all, get_opt, Rule};
use super::error::{self, Result};
use super::types::*;

pub struct Resolver {
    types: HashMap<String, Type>,
    namespace: Vec<String>,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            namespace: Vec::new(),
        }
    }

    pub fn resolve_type(&self, path: &Pair<Rule>) -> Result<Type> {
        debug!("Lookup type: {}", path.as_str());

        self.types
            .get(path.as_str())
            .map(|p| p.namespaced(path.as_str()))
            .ok_or(error::type_not_found(path))
    }

    pub fn resolve_generic_type(&self, p: &Pair<Rule>) -> Result<Type> {
        match get_opt(&p, Rule::Template) {
            Some(template) => {
                let mut tys = Vec::new();

                for gty in get_all(&template, Rule::GenericType) {
                    tys.push(self.resolve_generic_type(&gty)?);
                }

                let ident = get(&template, Rule::Identifier).as_str();

                Ok(Template::new(ident, tys).into())
            }
            None => {
                let ty = get(&p, Rule::Type);
                self.resolve_type(&ty)
            }
        }
    }

    pub fn add_type<T>(&mut self, ident: &str, ty: T)
    where
        T: Into<Type>,
    {
        let path = match self.namespace.last() {
            Some(namespace) => format!("{}::{}", namespace, ident),
            None => ident.to_string(),
        };

        debug!("Add type: {}", path);

        self.types.insert(path, ty.into());
    }

    pub fn enter_ns(&mut self, module: &str) {
        self.namespace.push(module.into());

        debug!("Entered to namespace: {}", module);
    }

    pub fn exit_ns(&mut self) {
        let _ns = self.namespace.pop();

        debug!("Exited to namespace: {}", _ns.unwrap_or("".into()));
    }
}
