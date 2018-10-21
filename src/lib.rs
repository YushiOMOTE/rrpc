extern crate pest;
#[macro_use]
extern crate pest_derive;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use pest::{Parser, Span};
use pest::error::{Error as PestError, ErrorVariant};
use pest::iterators::{Pair, Pairs};

#[derive(Parser)]
#[grammar = "rpc.pest"]
struct RpcParser;

use std::fs::File;
use std::io::prelude::*;
use std::fmt;

use serde_json::value::Value;
use serde_json::map::Map;
use std::collections::HashMap;

macro_rules! get {
    ($pair: ident, $variant: path) => {{
        let mut opt = None;
        for pair in $pair.clone().into_inner() {
            if let $variant = pair.as_rule() {
                opt = Some(pair)
            }
        }
        opt.ok_or(Error::bug(&$pair, $variant))
    }};
}

macro_rules! getm {
    ($pair: ident, $variant: path) => {{
        let mut array = Vec::new();
        for pair in $pair.clone().into_inner() {
            if let $variant = pair.as_rule() {
                array.push(pair);
            }
        }
        array
    }};
}

fn get_comment<'a>(p: &'a Pair<Rule>) -> Result<&'a str> {
    let p = get!(p, Rule::CommentLine)?;
    get!(p, Rule::Comment).map(|p| p.as_str())
}

#[derive(Debug)]
pub enum Error {
    FileError(String),
    TypeNotFound(PestError<Rule>),
    LoadError(PestError<Rule>),
    ParseError(PestError<Rule>),
    Bug(PestError<Rule>),
}

impl Error {
    fn file_error<T: ToString>(msg: T) -> Error {
        Error::FileError(msg.to_string())
    }

    fn type_not_found(p: &Pair<Rule>) -> Error {
        Error::TypeNotFound(PestError::new_from_span(
            ErrorVariant::CustomError {
                message: format!("Type not found: {}", p.as_str()),
            },
            p.as_span(),
        ))
    }

    fn load_error(p: &Pair<Rule>, path: &str, msg: &str) -> Error {
        Error::LoadError(PestError::new_from_span(
            ErrorVariant::CustomError {
                message: format!("Couldn't load module: {}: {}", path, msg),
            },
            p.as_span(),
        ))
    }

    fn parse_error(e: PestError<Rule>) -> Error {
        Error::ParseError(e)
    }

    fn bug(p: &Pair<Rule>, rule: Rule) -> Error {
        Error::TypeNotFound(PestError::new_from_span(
            ErrorVariant::CustomError {
                message: format!("Bug: missing {:?} in {}", rule, p.as_str()),
            },
            p.as_span(),
        ))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::FileError(e) => write!(f, "{}", e),
            Error::TypeNotFound(e) => write!(f, "{}", e),
            Error::LoadError(e) => write!(f, "{}", e),
            Error::ParseError(e) => write!(f, "{}", e),
            Error::Bug(e) => write!(f, "{}", e),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;
type Types = HashMap<String, Value>;

struct TypeResolver {
    types: HashMap<String, Value>,
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

impl TypeResolver {
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

        Self { types }
    }

    fn resolve(&self, path: &Pair<Rule>) -> Result<Value> {
        println!("Lookup type: {}", path.as_str());
        self.types
            .get(path.as_str())
            .map(|p| p.clone())
            .ok_or(Error::type_not_found(path))
    }

    fn register(&mut self, path: &str, value: Value) {
        println!("Add type: {}: {}", path, value);
        self.types.insert(path.into(), value);
    }
}

fn parse(pairs: Pairs<Rule>, types: &mut TypeResolver) -> Result<Value> {
    let mut uses = Vec::new();
    let mut module = None;

    for p in pairs {
        match p.as_rule() {
            Rule::Use => {
                uses.push(parse_use(p, types)?);
            }
            Rule::Module => {
                module = Some(parse_module(p, types)?);
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    Ok(json!({
        "uses": uses,
        "mod": module.expect("Module not found"),
    }))
}

fn path(module: &str, ident: &str) -> String {
    format!("{}::{}", module, ident)
}

fn parse_root(path: &str) -> Result<Value> {
    let mut file = File::open(path).map_err(|e| Error::file_error(e))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| Error::file_error(e))?;

    let pairs = RpcParser::parse(Rule::File, &contents).map_err(|e| Error::parse_error(e))?;
    let mut types = TypeResolver::new();
    parse(pairs, &mut types)
}

fn parse_use(p: Pair<Rule>, types: &mut TypeResolver) -> Result<Value> {
    let path = getm!(p, Rule::Path);
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

    let mut file =
        File::open(path.clone()).map_err(|e| Error::load_error(&p, &path, &e.to_string()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| Error::load_error(&p, &path, &e.to_string()))?;

    let pairs = RpcParser::parse(Rule::File, &contents).map_err(|e| Error::parse_error(e))?;
    let _ = parse(pairs, types)?;

    Ok(json!({
        "module": raw_path,
        "path": path,
    }))
}

fn parse_module(p: Pair<Rule>, types: &mut TypeResolver) -> Result<Value> {
    let module = get!(p, Rule::Identifier)?.as_str();
    let mut nodes = Vec::new();

    for p in p.into_inner() {
        match p.as_rule() {
            Rule::Struct => {
                let (ident, value) = parse_struct(p, types)?;

                types.register(&path(module, &ident), value.clone());

                nodes.push(value);
            }
            Rule::Enum => {
                let (ident, value) = parse_enum(p, types)?;

                types.register(&path(module, &ident), value.clone());

                nodes.push(value);
            }
            Rule::Interface => nodes.push(parse_interface(p, types)?),
            Rule::Identifier => {}
            _ => unreachable!(),
        }
    }

    Ok(json!({
        "name": module,
        "nodes": nodes,
    }))
}

fn resolve_generic_type(p: Pair<Rule>, types: &TypeResolver) -> Result<Value> {
    match get!(p, Rule::Template).ok() {
        Some(template) => {
            let mut tys = Vec::new();

            for gty in getm!(template, Rule::GenericType) {
                tys.push(resolve_generic_type(gty, types)?);
            }

            let ident = get!(template, Rule::Identifier)?.as_str();

            Ok(json!({
                "name": ident,
                "type": "template",
                "subtypes": tys,
            }))
        }
        None => {
            let ty = get!(p, Rule::Type)?;
            types.resolve(&ty)
        }
    }
}

fn parse_struct(p: Pair<Rule>, types: &mut TypeResolver) -> Result<(String, Value)> {
    let mut fields = Vec::new();

    for f in getm!(p, Rule::Field) {
        let comment = get_comment(&f).ok();
        let ident = get!(f, Rule::Identifier)?.as_str();
        let gty = get!(f, Rule::GenericType)?;
        let value = get!(f, Rule::Value).ok().map(|p| p.as_str());

        fields.push(json!({
            "comment": comment,
            "name": ident,
            "type": resolve_generic_type(gty, types)?,
            "value": value,
        }));
    }

    let comment = get_comment(&p).ok();
    let ident = get!(p, Rule::Identifier)?.as_str();

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

fn parse_enum(p: Pair<Rule>, types: &mut TypeResolver) -> Result<(String, Value)> {
    let mut fields = Vec::new();

    let uty = get!(p, Rule::Type)?;

    for f in getm!(p, Rule::Variant) {
        let comment = get_comment(&f).ok();
        let ident = get!(f, Rule::Identifier)?.as_str();
        let value = get!(f, Rule::Value).ok().map(|p| p.as_str());

        fields.push(json!({
            "comment": comment,
            "name": ident,
            "type": types.resolve(&uty)?,
            "value": value,
        }));
    }

    let comment = get_comment(&p).ok();
    let ident = get!(p, Rule::Identifier)?.as_str();

    Ok((
        ident.into(),
        json!({
            "comment" : comment,
            "name": ident,
            "type": types.resolve(&uty)?,
            "fields": fields,
        }),
    ))
}

fn parse_interface(p: Pair<Rule>, types: &mut TypeResolver) -> Result<Value> {
    let mut funcs = Vec::new();

    for f in getm!(p, Rule::Function) {
        let comment = get_comment(&p).ok();
        let ident = get!(f, Rule::Identifier)?.as_str();
        let mut args = Vec::new();

        for a in getm!(f, Rule::Argument) {
            let ident = get!(a, Rule::Identifier)?.as_str();
            let ty = get!(a, Rule::Type)?;

            args.push(json!({
                    "name": ident,
                    "type": types.resolve(&ty)?,
                }));
        }

        let r = get!(f, Rule::ReturnType).ok();
        let r = if let Some(r) = r {
            let mut rs = Vec::new();

            for ty in getm!(r, Rule::Type) {
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
    let ident = get!(p, Rule::Identifier)?.as_str();
    let pattern = get!(p, Rule::Pattern)?.as_str();

    Ok(json!({
        "comment" : comment,
        "name": ident,
        "pattern": pattern,
        "type": "interface",
        "funcs": funcs,
    }))
}

pub fn run() {
    let j = parse_root("examples/init.rpc")
        .map_err(|e| panic!("{}", e))
        .unwrap();
    println!("{}", serde_json::to_string_pretty(&j).unwrap());

    // let mut ast = Ast::new();

    // for pair in pairs {
    //     match pair.as_rule() {
    //         Rule::Use => ast.add_use(&pair),
    //         Rule::Module => {}
    //         Rule::EOI => {}
    //         _ => unreachable!(),
    //     }
    //     println!("{:#?}", pair.as_rule());
    // }
}
