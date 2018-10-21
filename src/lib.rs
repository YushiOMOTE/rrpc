extern crate pest;
#[macro_use]
extern crate pest_derive;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate serde_derive;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;

use pest::Parser;
use pest::error::{Error as PestError, ErrorVariant};
use pest::iterators::{Pair, Pairs};

#[derive(Parser)]
#[grammar = "rpc.pest"]
struct RpcParser;

use std::fs::File;
use std::io::prelude::*;
use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::value::Value;
use std::collections::HashMap;

fn get<'a>(p: &Pair<'a, Rule>, rule: Rule) -> Result<Pair<'a, Rule>> {
    let mut opt = None;
    for pair in p.clone().into_inner() {
        if rule == pair.as_rule() {
            opt = Some(pair)
        }
    }
    opt.ok_or(InternalError::bug(&p, rule))
}

fn get_all<'a>(p: &Pair<'a, Rule>, rule: Rule) -> Vec<Pair<'a, Rule>> {
    let mut array = Vec::new();
    for pair in p.clone().into_inner() {
        if rule == pair.as_rule() {
            array.push(pair);
        }
    }
    array
}

fn get_comment<'a>(p: &'a Pair<Rule>) -> Result<&'a str> {
    let p = get(p, Rule::CommentLine)?;
    get(&p, Rule::Comment).map(|p| p.as_str())
}

#[derive(Debug)]
pub enum InternalError {
    FileError(String),
    TypeNotFound(PestError<Rule>),
    LoadError(PestError<Rule>),
    ParseError(PestError<Rule>),
    Bug(PestError<Rule>),
}

error_chain! {
    foreign_links {
        Internal(InternalError);
    }
}

impl InternalError {
    fn file_error<T: ToString>(msg: T) -> Error {
        InternalError::FileError(msg.to_string()).into()
    }

    fn type_not_found(p: &Pair<Rule>) -> Error {
        InternalError::TypeNotFound(PestError::new_from_span(
            ErrorVariant::CustomError {
                message: format!("Type not found: {}", p.as_str()),
            },
            p.as_span(),
        )).into()
    }

    fn load_error(p: &Pair<Rule>, path: &str) -> Error {
        InternalError::LoadError(PestError::new_from_span(
            ErrorVariant::CustomError {
                message: format!("Couldn't load module: {}", path),
            },
            p.as_span(),
        )).into()
    }

    fn parse_error(e: PestError<Rule>) -> Error {
        InternalError::ParseError(e).into()
    }

    fn bug(p: &Pair<Rule>, rule: Rule) -> Error {
        InternalError::Bug(PestError::new_from_span(
            ErrorVariant::CustomError {
                message: format!("Bug: missing {:?} in {}", rule, p.as_str()),
            },
            p.as_span(),
        )).into()
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InternalError::FileError(e) => write!(f, "{}", e),
            InternalError::TypeNotFound(e) => write!(f, "{}", e),
            InternalError::LoadError(e) => write!(f, "{}", e),
            InternalError::ParseError(e) => write!(f, "{}", e),
            InternalError::Bug(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for InternalError {
    fn description(&self) -> &str {
        "compile error"
    }

    fn cause(&self) -> Option<&std::error::Error> {
        None
    }
}

struct Resolver {
    types: HashMap<String, Value>,
    namespace: Vec<String>,
    directory: Vec<PathBuf>,
}

fn primitive(types: &mut HashMap<String, Value>, name: &str) {
    types.insert(
        name.into(),
        json!({
        "name": name,
        "type": "primitive",
    }),
    );
}

impl Resolver {
    fn new() -> Self {
        let mut types = HashMap::new();

        primitive(&mut types, "bool");
        primitive(&mut types, "u8");
        primitive(&mut types, "u16");
        primitive(&mut types, "u32");
        primitive(&mut types, "u64");
        primitive(&mut types, "i8");
        primitive(&mut types, "i16");
        primitive(&mut types, "i32");
        primitive(&mut types, "i64");
        primitive(&mut types, "f32");
        primitive(&mut types, "f64");
        primitive(&mut types, "string");

        Self {
            types,
            namespace: Vec::new(),
            directory: Vec::new(),
        }
    }

    fn resolve_type(&self, path: &Pair<Rule>) -> Result<Value> {
        debug!("Lookup type: {}", path.as_str());

        self.types
            .get(path.as_str())
            .map(|p| p.clone())
            .ok_or(InternalError::type_not_found(path))
    }

    fn resolve_generic_type(&self, p: Pair<Rule>) -> Result<Value> {
        match get(&p, Rule::Template).ok() {
            Some(template) => {
                let mut tys = Vec::new();

                for gty in get_all(&template, Rule::GenericType) {
                    tys.push(self.resolve_generic_type(gty)?);
                }

                let ident = get(&template, Rule::Identifier)?.as_str();

                Ok(json!({
                    "name": ident,
                    "type": "template",
                    "subtypes": tys,
                }))
            }
            None => {
                let ty = get(&p, Rule::Type)?;
                self.resolve_type(&ty)
            }
        }
    }

    fn add_type(&mut self, ident: &str, value: Value) {
        let path = match self.namespace.last() {
            Some(namespace) => format!("{}::{}", namespace, ident),
            None => ident.to_string(),
        };

        debug!("Add type: {}", path);

        self.types.insert(path, value);
    }

    fn load(&self, path: &str) -> Result<String> {
        let path = Path::new(self.current_dir()).join(path);

        let mut file = File::open(path).map_err(|e| InternalError::file_error(e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| InternalError::file_error(e))?;

        Ok(contents)
    }

    fn enter_dir(&mut self, dir: &str) -> Result<()> {
        let path = if let Some(path) = self.directory.last() {
            path.join(dir)
        } else {
            Path::new(dir).to_path_buf()
        };
        let mut path = path.canonicalize()
            .map_err(|e| InternalError::file_error(e))?;

        path.pop();

        self.directory.push(path);

        Ok(())
    }

    fn exit_dir(&mut self) {
        self.directory.pop();
    }

    fn current_dir(&self) -> &str {
        self.directory.last().and_then(|p| p.to_str()).unwrap_or("")
    }

    fn enter_ns(&mut self, module: &str) {
        self.namespace.push(module.into());
    }

    fn exit_ns(&mut self) {
        self.namespace.pop();
    }
}

fn parse<'a>(s: &'a str) -> Result<Pairs<'a, Rule>> {
    RpcParser::parse(Rule::File, s).map_err(|e| InternalError::parse_error(e))
}

fn generate_defs(pairs: Pairs<Rule>, resolver: &mut Resolver) -> Result<Value> {
    let mut uses = Vec::new();
    let mut nodes = Vec::new();

    for p in pairs {
        match p.as_rule() {
            Rule::Use => {
                uses.push(generate_use(p, resolver)?);
            }
            Rule::Struct => {
                let (ident, value) = generate_struct(p, resolver)?;

                resolver.add_type(&ident, value.clone());

                nodes.push(value);
            }
            Rule::Enum => {
                let (ident, value) = generate_enum(p, resolver)?;

                resolver.add_type(&ident, value.clone());

                nodes.push(value);
            }
            Rule::Interface => nodes.push(generate_interface(p, resolver)?),
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    Ok(json!({
        "uses": uses,
        "nodes": nodes,
    }))
}

fn generate(path: &str) -> Result<Value> {
    let mut resolver = Resolver::new();

    let contents = resolver.load(path)?;
    let pairs = parse(&contents)?;

    resolver.enter_dir(path)?;
    generate_defs(pairs, &mut resolver)
}

fn generate_use(p: Pair<Rule>, resolver: &mut Resolver) -> Result<Value> {
    let path = get_all(&p, Rule::Path);
    let raw_path = path.clone()
        .into_iter()
        .map(|p| p.as_str())
        .collect::<Vec<_>>()
        .join("::");
    let path = path.into_iter()
        .map(|p| p.as_str())
        .collect::<Vec<_>>()
        .join("/");
    let path = format!("{}.rpc", path);

    let contents = resolver
        .load(&path)
        .chain_err(|| InternalError::load_error(&p, &path))?;
    let pairs = parse(&contents).chain_err(|| InternalError::load_error(&p, &path))?;

    resolver.enter_dir(&path)?;
    resolver.enter_ns(&raw_path);
    let _ = generate_defs(pairs, resolver).chain_err(|| InternalError::load_error(&p, &path))?;
    resolver.exit_ns();
    resolver.exit_dir();

    Ok(json!({
        "module": raw_path,
        "path": path,
    }))
}

fn generate_struct(p: Pair<Rule>, resolver: &mut Resolver) -> Result<(String, Value)> {
    let mut fields = Vec::new();

    for f in get_all(&p, Rule::Field) {
        let comment = get_comment(&f).ok();
        let ident = get(&f, Rule::Identifier)?.as_str();
        let gty = get(&f, Rule::GenericType)?;
        let value = get(&f, Rule::Value).ok().map(|p| p.as_str());

        fields.push(json!({
            "comment": comment,
            "name": ident,
            "type": resolver.resolve_generic_type(gty)?,
            "value": value,
        }));
    }

    let comment = get_comment(&p).ok();
    let ident = get(&p, Rule::Identifier)?.as_str();

    Ok((
        ident.into(),
        json!({
            "comment" : comment,
            "name": ident,
            "type": "struct",
            "fields": fields,
        }),
    ))
}

fn generate_enum(p: Pair<Rule>, resolver: &mut Resolver) -> Result<(String, Value)> {
    let mut fields = Vec::new();

    let uty = get(&p, Rule::Type)?;

    for f in get_all(&p, Rule::Variant) {
        let comment = get_comment(&f).ok();
        let ident = get(&f, Rule::Identifier)?.as_str();
        let value = get(&f, Rule::Value).ok().map(|p| p.as_str());

        fields.push(json!({
            "comment": comment,
            "name": ident,
            "type": resolver.resolve_type(&uty)?,
            "value": value,
        }));
    }

    let comment = get_comment(&p).ok();
    let ident = get(&p, Rule::Identifier)?.as_str();

    Ok((
        ident.into(),
        json!({
            "comment" : comment,
            "name": ident,
            "type": resolver.resolve_type(&uty)?,
            "fields": fields,
        }),
    ))
}

fn generate_interface(p: Pair<Rule>, resolver: &mut Resolver) -> Result<Value> {
    let mut funcs = Vec::new();

    for f in get_all(&p, Rule::Function) {
        let comment = get_comment(&p).ok();
        let ident = get(&f, Rule::Identifier)?.as_str();
        let mut args = Vec::new();

        for a in get_all(&f, Rule::Argument) {
            let ident = get(&a, Rule::Identifier)?.as_str();
            let ty = get(&a, Rule::Type)?;

            args.push(json!({
                    "name": ident,
                    "type": resolver.resolve_type(&ty)?,
                }));
        }

        let r = get(&f, Rule::ReturnType).ok();
        let r = if let Some(r) = r {
            let mut rs = Vec::new();

            for ty in get_all(&r, Rule::Type) {
                rs.push(json!(ty.as_str()));
            }

            Some(rs)
        } else {
            None
        };

        funcs.push(json!({
            "comment": comment,
            "name": ident,
            "args": args,
            "return": r,
        }));
    }

    let comment = get_comment(&p).ok();
    let ident = get(&p, Rule::Identifier)?.as_str();
    let pattern = get(&p, Rule::Pattern)?.as_str();

    Ok(json!({
        "comment" : comment,
        "name": ident,
        "pattern": pattern,
        "type": "interface",
        "funcs": funcs,
    }))
}

use error_chain::ChainedError;

pub fn run() {
    let j = generate("examples/init.rpc")
        .map_err(|e| panic!("{}", e.display_chain().to_string()))
        .unwrap();
    println!("{}", serde_json::to_string_pretty(&j).unwrap());
}
