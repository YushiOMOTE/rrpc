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
use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::value::Value;

use error_chain::ChainedError;

fn get<'a>(p: &Pair<'a, Rule>, rule: Rule) -> Pair<'a, Rule> {
    match get_opt(p, rule) {
        Some(p) => p,
        None => panic!("missing {:?} in {}", rule, p.as_str()),
    }
}

fn get_opt<'a>(p: &Pair<'a, Rule>, rule: Rule) -> Option<Pair<'a, Rule>> {
    for pair in p.clone().into_inner() {
        if rule == pair.as_rule() {
            return Some(pair);
        }
    }
    None
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

fn get_comment<'a>(p: &'a Pair<Rule>) -> Option<&'a str> {
    let p = get_opt(p, Rule::CommentLine)?;
    get_opt(&p, Rule::Comment).map(|p| p.as_str())
}

#[derive(Debug)]
pub enum InternalError {
    FileError(String),
    TypeNotFound(String, PestError<Rule>),
    LoadError(String, PestError<Rule>),
    ParseError(String, PestError<Rule>),
    Duplicated(String, PestError<Rule>),
}

error_chain! {
    foreign_links {
        Internal(InternalError);
    }
}

impl InternalError {
    fn file_error<T: ToString>(e: T) -> Error {
        InternalError::FileError(e.to_string()).into()
    }

    fn type_not_found(path: &str, p: &Pair<Rule>) -> Error {
        InternalError::TypeNotFound(
            path.into(),
            PestError::new_from_span(
                ErrorVariant::CustomError {
                    message: format!("type not found: {}", p.as_str()),
                },
                p.as_span(),
            ),
        ).into()
    }

    fn load_error(path: &str, p: &Pair<Rule>, module: &str) -> Error {
        InternalError::LoadError(
            path.into(),
            PestError::new_from_span(
                ErrorVariant::CustomError {
                    message: format!("couldn't load module: {}", module),
                },
                p.as_span(),
            ),
        ).into()
    }

    fn duplicated(name: &str, path: &str, p: &Pair<Rule>) -> Error {
        InternalError::Duplicated(
            path.into(),
            PestError::new_from_span(
                ErrorVariant::CustomError {
                    message: format!("duplicated {}: {}", name, p.as_str()),
                },
                p.as_span(),
            ),
        ).into()
    }

    fn parse_error(path: &str, e: PestError<Rule>) -> Error {
        InternalError::ParseError(path.into(), e).into()
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InternalError::FileError(e) => write!(f, "{}", e),
            InternalError::TypeNotFound(path, e) => write!(f, "{}:\n {}", path, e),
            InternalError::LoadError(path, e) => write!(f, "{}:\n {}", path, e),
            InternalError::ParseError(path, e) => write!(f, "{}:\n {}", path, e),
            InternalError::Duplicated(path, e) => write!(f, "{}:\n {}", path, e),
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

struct DupChecker<'a> {
    name: &'a str,
    path: &'a str,
    set: HashSet<String>,
}

impl<'a> DupChecker<'a> {
    fn new(name: &'a str, path: &'a str) -> DupChecker<'a> {
        DupChecker {
            name,
            path: path.into(),
            set: HashSet::new(),
        }
    }

    fn check(&mut self, p: &Pair<Rule>) -> Result<()> {
        if self.set.insert(p.as_str().into()) {
            Ok(())
        } else {
            Err(InternalError::duplicated(self.name, self.path, p))
        }
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
            .ok_or(InternalError::type_not_found(self.current_file(), path))
    }

    fn resolve_generic_type(&self, p: Pair<Rule>) -> Result<Value> {
        match get_opt(&p, Rule::Template) {
            Some(template) => {
                let mut tys = Vec::new();

                for gty in get_all(&template, Rule::GenericType) {
                    tys.push(self.resolve_generic_type(gty)?);
                }

                let ident = get(&template, Rule::Identifier).as_str();

                Ok(json!({
                    "name": ident,
                    "type": "template",
                    "subtypes": tys,
                }))
            }
            None => {
                let ty = get(&p, Rule::Type);
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
        debug!("Loading file: {}", path);

        let path = Path::new(&self.current_dir()).join(path);

        debug!("Loading path: {}", path.to_string_lossy());

        let mut file = File::open(path).map_err(|e| InternalError::file_error(e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| InternalError::file_error(e))?;

        Ok(contents)
    }

    fn enter_dir(&mut self, dir: &str) -> Result<()> {
        let path = Path::new(&self.current_dir()).join(dir);

        let path = path.canonicalize()
            .map_err(|e| InternalError::file_error(e))?;

        self.directory.push(path);

        debug!("Entered to directory: {}", self.current_dir());

        Ok(())
    }

    fn exit_dir(&mut self) {
        self.directory.pop();

        debug!("Exited to directory: {}", self.current_dir());
    }

    fn current_file(&self) -> &str {
        self.directory.last().and_then(|p| p.to_str()).unwrap_or("")
    }

    fn current_dir(&self) -> String {
        self.directory
            .last()
            .map(|p| {
                let mut p = p.clone();
                p.pop();
                p.to_string_lossy().to_string()
            })
            .unwrap_or("".into())
    }

    fn enter_ns(&mut self, module: &str) {
        self.namespace.push(module.into());

        debug!("Entered to namespace: {}", module);
    }

    fn exit_ns(&mut self) {
        let _ns = self.namespace.pop();

        debug!("Exited to namespace: {}", _ns.unwrap_or("".into()));
    }
}

fn parse<'a>(path: &str, s: &'a str) -> Result<Pairs<'a, Rule>> {
    RpcParser::parse(Rule::File, s).map_err(|e| InternalError::parse_error(path.into(), e))
}

fn generate_defs(pairs: Pairs<Rule>, resolver: &mut Resolver) -> Result<Value> {
    let mut uses = Vec::new();
    let mut nodes = Vec::new();

    let current = resolver.current_file().to_string();

    let mut ty_checker = DupChecker::new("type name", &current);
    let mut if_checker = DupChecker::new("interface name", &current);

    for p in pairs {
        match p.as_rule() {
            Rule::Use => {
                uses.push(generate_use(p, resolver)?);
            }
            Rule::Struct => {
                let (ident, value) = generate_struct(p, resolver)?;

                ty_checker.check(&ident)?;

                resolver.add_type(ident.as_str(), value.clone());

                nodes.push(value);
            }
            Rule::Enum => {
                let (ident, value) = generate_enum(p, resolver)?;

                ty_checker.check(&ident)?;

                resolver.add_type(ident.as_str(), value.clone());

                nodes.push(value);
            }
            Rule::Interface => {
                let (ident, value) = generate_interface(p, resolver)?;

                if_checker.check(&ident)?;

                nodes.push(value);
            }
            Rule::EOI => {}
            _ => unreachable!("unexpected token {:?}", p),
        }
    }

    Ok(json!({
        "uses": uses,
        "nodes": nodes,
    }))
}

fn generate(path: &str) -> Result<Value> {
    debug!("Generating from {}", path);

    let mut resolver = Resolver::new();

    let contents = resolver.load(path)?;

    resolver.enter_dir(path)?;

    let pairs = parse(resolver.current_file(), &contents)?;
    generate_defs(pairs, &mut resolver)
}

fn generate_use(p: Pair<Rule>, resolver: &mut Resolver) -> Result<Value> {
    trace!("Generating use:\n {}", p.as_str());

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
        .chain_err(|| InternalError::load_error(resolver.current_file(), &p, &path))?;
    let pairs = parse(resolver.current_file(), &contents)
        .chain_err(|| InternalError::load_error(resolver.current_file(), &p, &path))?;

    resolver
        .enter_dir(&path)
        .chain_err(|| InternalError::load_error(resolver.current_file(), &p, &path))?;
    resolver.enter_ns(&raw_path);
    let _ = generate_defs(pairs, resolver)
        .chain_err(|| InternalError::load_error(resolver.current_file(), &p, &path))?;
    resolver.exit_ns();
    resolver.exit_dir();

    Ok(json!({
        "module": raw_path,
        "path": path,
    }))
}

fn generate_struct<'a>(
    p: Pair<'a, Rule>,
    resolver: &mut Resolver,
) -> Result<(Pair<'a, Rule>, Value)> {
    trace!("Generating struct:\n {}", p.as_str());

    let mut checker = DupChecker::new("struct member name", &resolver.current_file());

    let mut fields = Vec::new();

    for f in get_all(&p, Rule::Field) {
        let comment = get_comment(&f);
        let ident = get(&f, Rule::Identifier);
        let gty = get(&f, Rule::GenericType);
        let value = get_opt(&f, Rule::Value).map(|p| p.as_str());

        checker.check(&ident)?;

        fields.push(json!({
            "comment": comment,
            "name": ident.as_str(),
            "type": resolver.resolve_generic_type(gty)?,
            "value": value,
        }));
    }

    let comment = get_comment(&p);
    let ident = get(&p, Rule::Identifier);

    Ok((
        ident.clone(),
        json!({
            "comment" : comment,
            "name": ident.as_str(),
            "type": "struct",
            "fields": fields,
        }),
    ))
}

fn generate_enum<'a>(
    p: Pair<'a, Rule>,
    resolver: &mut Resolver,
) -> Result<(Pair<'a, Rule>, Value)> {
    trace!("Generating enum:\n {}", p.as_str());

    let mut checker = DupChecker::new("enum variant name", &resolver.current_file());

    let mut fields = Vec::new();

    let uty = get(&p, Rule::Type);

    for f in get_all(&p, Rule::Variant) {
        let comment = get_comment(&f);
        let ident = get(&f, Rule::Identifier);
        let value = get_opt(&f, Rule::Value).map(|p| p.as_str());

        checker.check(&ident)?;

        fields.push(json!({
            "comment": comment,
            "name": ident.as_str(),
            "type": resolver.resolve_type(&uty)?,
            "value": value,
        }));
    }

    let comment = get_comment(&p);
    let ident = get(&p, Rule::Identifier);

    Ok((
        ident.clone(),
        json!({
            "comment" : comment,
            "name": ident.as_str(),
            "type": resolver.resolve_type(&uty)?,
            "fields": fields,
        }),
    ))
}

fn generate_interface<'a>(
    p: Pair<'a, Rule>,
    resolver: &mut Resolver,
) -> Result<(Pair<'a, Rule>, Value)> {
    trace!("Generating interface:\n {}", p.as_str());

    let mut funcs = Vec::new();

    let mut checker = DupChecker::new("function name", &resolver.current_file());

    for f in get_all(&p, Rule::Function) {
        let comment = get_comment(&p);
        let ident = get(&f, Rule::Identifier);
        let mut args = Vec::new();

        checker.check(&ident)?;

        let mut arg_checker = DupChecker::new("argument name", &resolver.current_file());

        for a in get_all(&f, Rule::Argument) {
            let ident = get(&a, Rule::Identifier);
            let ty = get(&a, Rule::Type);

            arg_checker.check(&ident)?;

            args.push(json!({
                    "name": ident.as_str(),
                    "type": resolver.resolve_type(&ty)?,
                }));
        }

        let r = get_opt(&f, Rule::ReturnType);
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
            "name": ident.as_str(),
            "args": args,
            "return": r,
        }));
    }

    let comment = get_comment(&p);
    let ident = get(&p, Rule::Identifier);
    let pattern = get(&p, Rule::Pattern).as_str();

    Ok((
        ident.clone(),
        json!({
        "comment" : comment,
        "name": ident.as_str(),
        "pattern": pattern,
        "type": "interface",
        "funcs": funcs,
    }),
    ))
}

pub fn run() {
    let j = match generate("examples/init.rpc") {
        Ok(j) => j,
        Err(e) => return error!("{}", e.display_chain().to_string()),
    };
    println!("{}", serde_json::to_string_pretty(&j).unwrap());
}
