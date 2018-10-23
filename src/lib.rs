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

use pest::Parser;
use pest::error::{Error as PestError, ErrorVariant};
use pest::iterators::{Pair, Pairs};

#[derive(Parser)]
#[grammar = "rpc.pest"]
struct RpcParser;

use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::value::Value;

mod types;

use self::types::*;

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

error_chain! {
    errors {
        Error(path: String) {
            description("compile error")
                display("{}", path)
        }

        FileError(e: String) {
            description("i/o error")
                display("{}", e)
        }

        ValueError(e: PestError<Rule>) {
            description("compile erorr")
                display("{}", e)
        }

        TypeNotFound(e: PestError<Rule>) {
            description("compile error")
                display("{}", e)
        }

        LoadError(e: PestError<Rule>) {
            description("compile error")
                display("{}", e)
        }

        ParseError(e: PestError<Rule>) {
            description("compile error")
                display("{}", e)
        }

        Duplicated(e: PestError<Rule>) {
            description("compile error")
                display("{}", e)
        }

        PackError(e: serde_json::error::Error) {
            description("compile error")
                display("{}", e)
        }
    }
}

fn error(path: &str) -> Error {
    ErrorKind::Error(path.to_string()).into()
}

fn file_error<T: ToString>(e: T) -> Error {
    ErrorKind::FileError(e.to_string()).into()
}

fn value_error<T: ToString>(p: &Pair<Rule>, e: T) -> Error {
    ErrorKind::ValueError(PestError::new_from_span(
        ErrorVariant::CustomError {
            message: format!("value needs to be a valid json: {}", e.to_string()),
        },
        p.as_span(),
    )).into()
}

fn type_not_found(p: &Pair<Rule>) -> Error {
    ErrorKind::TypeNotFound(PestError::new_from_span(
        ErrorVariant::CustomError {
            message: format!("type not found: {}", p.as_str()),
        },
        p.as_span(),
    )).into()
}

fn load_error(p: &Pair<Rule>, module: &str) -> Error {
    ErrorKind::LoadError(PestError::new_from_span(
        ErrorVariant::CustomError {
            message: format!("couldn't load module: {}", module),
        },
        p.as_span(),
    )).into()
}

fn duplicated(name: &str, p: &Pair<Rule>) -> Error {
    ErrorKind::Duplicated(PestError::new_from_span(
        ErrorVariant::CustomError {
            message: format!("duplicated {}: {}", name, p.as_str()),
        },
        p.as_span(),
    )).into()
}

fn parse_error(e: PestError<Rule>) -> Error {
    ErrorKind::ParseError(e).into()
}

fn pack_error(e: serde_json::error::Error) -> Error {
    ErrorKind::PackError(e).into()
}

struct DupChecker<'a> {
    name: &'a str,
    set: HashSet<String>,
}

impl<'a> DupChecker<'a> {
    fn new(name: &'a str) -> DupChecker<'a> {
        DupChecker {
            name,
            set: HashSet::new(),
        }
    }

    fn check(&mut self, p: &Pair<Rule>) -> Result<()> {
        if self.set.insert(p.as_str().into()) {
            Ok(())
        } else {
            Err(duplicated(self.name, p))
        }
    }
}

struct Loader {
    directory: Vec<PathBuf>,
}

impl Loader {
    fn new() -> Self {
        Self {
            directory: Vec::new(),
        }
    }

    fn load(&self, path: &str) -> Result<String> {
        debug!("Loading file: {}", path);

        let path = Path::new(&self.current_dir()).join(path);

        debug!("Loading path: {}", path.to_string_lossy());

        let mut file = File::open(path).map_err(|e| file_error(e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| file_error(e))?;

        Ok(contents)
    }

    fn enter_dir(&mut self, dir: &str) -> Result<()> {
        let path = Path::new(&self.current_dir()).join(dir);

        let path = path.canonicalize().map_err(|e| file_error(e))?;

        self.directory.push(path);

        debug!("Entered to directory: {}", self.current_dir());

        Ok(())
    }

    fn exit_dir(&mut self) {
        self.directory.pop();

        debug!("Exited to directory: {}", self.current_dir());
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
}

fn primitive(resolver: &mut Resolver, lang: &mut LangGenerator, ident: &str, tt: Trait) {
    resolver.add_type(
        ident.into(),
        lang.generate_primitive(Primitive::new(ident, tt)),
    );
}

struct Resolver {
    types: HashMap<String, Type>,
    namespace: Vec<String>,
}

impl Resolver {
    fn new() -> Self {
        Self {
            types: HashMap::new(),
            namespace: Vec::new(),
        }
    }

    fn resolve_type(&self, path: &Pair<Rule>) -> Result<Type> {
        debug!("Lookup type: {}", path.as_str());

        self.types
            .get(path.as_str())
            .map(|p| p.namespaced(path.as_str()))
            .ok_or(type_not_found(path))
    }

    fn resolve_generic_type(&self, p: &Pair<Rule>) -> Result<Type> {
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

    fn add_type<T>(&mut self, ident: &str, ty: T)
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

    fn enter_ns(&mut self, module: &str) {
        self.namespace.push(module.into());

        debug!("Entered to namespace: {}", module);
    }

    fn exit_ns(&mut self) {
        let _ns = self.namespace.pop();

        debug!("Exited to namespace: {}", _ns.unwrap_or("".into()));
    }
}

fn parse<'a>(s: &'a str) -> Result<Pairs<'a, Rule>> {
    RpcParser::parse(Rule::File, s).map_err(|e| parse_error(e))
}

fn parse_value(p: &Pair<Rule>) -> Result<Option<Value>> {
    match get_opt(p, Rule::Value).map(|p| p.as_str()) {
        Some(s) => Ok(Some(serde_json::from_str(s).map_err(|e| value_error(p, e))?)),
        None => Ok(None),
    }
}

pub trait LangGenerator {
    fn generate_primitive(&mut self, value: Primitive) -> Primitive {
        value
    }

    fn generate_use(&mut self, value: Use) -> Result<Use> {
        Ok(value)
    }

    fn generate_field(&mut self, value: Field) -> Result<Field> {
        Ok(value)
    }

    fn generate_enum(&mut self, value: Enum) -> Result<Enum> {
        Ok(value)
    }

    fn generate_variant(&mut self, value: Variant) -> Result<Variant> {
        Ok(value)
    }

    fn generate_arg(&mut self, value: Arg) -> Result<Arg> {
        Ok(value)
    }

    fn generate_struct(&mut self, value: Struct) -> Result<Struct> {
        Ok(value)
    }

    fn generate_func(&mut self, value: Func) -> Result<Func> {
        Ok(value)
    }

    fn generate_interface(&mut self, value: Interface) -> Result<Interface> {
        Ok(value)
    }

    fn generate_defs(&mut self, value: Defs) -> Result<Defs> {
        Ok(value)
    }
}

struct NullGenerator;

impl LangGenerator for NullGenerator {}

struct Generator<'g> {
    resolver: Resolver,
    loader: Loader,
    lang: &'g mut LangGenerator,
}

impl<'g> Generator<'g> {
    fn new(lang: &'g mut LangGenerator) -> Self {
        let mut resolver = Resolver::new();

        primitive(&mut resolver, lang, "bool", Trait::Bool);
        primitive(&mut resolver, lang, "u8", Trait::Integer);
        primitive(&mut resolver, lang, "u16", Trait::Integer);
        primitive(&mut resolver, lang, "u32", Trait::Integer);
        primitive(&mut resolver, lang, "u64", Trait::Integer);
        primitive(&mut resolver, lang, "i8", Trait::Integer);
        primitive(&mut resolver, lang, "i16", Trait::Integer);
        primitive(&mut resolver, lang, "i32", Trait::Integer);
        primitive(&mut resolver, lang, "i64", Trait::Integer);
        primitive(&mut resolver, lang, "f32", Trait::Float);
        primitive(&mut resolver, lang, "f64", Trait::Float);
        primitive(&mut resolver, lang, "string", Trait::String);

        Self {
            resolver,
            loader: Loader::new(),
            lang,
        }
    }

    fn generate(&mut self, path: &str) -> Result<Defs> {
        debug!("Generating from {}", path);

        let contents = self.loader.load(path)?;

        self.loader.enter_dir(path)?;

        let pairs = parse(&contents)?;
        let model = self.generate_defs(pairs)?;

        self.loader.exit_dir();

        Ok(model)
    }

    fn load_submodule(&mut self, path: &str, ns: &str) -> Result<()> {
        debug!("Loading submodule: {} ({})", ns, path);

        let contents = self.loader.load(path)?;

        self.loader.enter_dir(&path)?;
        self.resolver.enter_ns(&ns);

        let pairs = parse(&contents)?;
        let _ = self.generate_defs(pairs)?;

        self.resolver.exit_ns();
        self.loader.exit_dir();

        Ok(())
    }

    fn generate_defs(&mut self, pairs: Pairs<Rule>) -> Result<Defs> {
        let mut uses = Vec::new();
        let mut nodes = Vec::new();

        let mut ty_checker = DupChecker::new("type name");
        let mut if_checker = DupChecker::new("interface name");

        for p in pairs {
            match p.as_rule() {
                Rule::Use => {
                    uses.push(self.generate_use(p)?);
                }
                Rule::Struct => {
                    let (ident, value) = self.generate_struct(p)?;

                    ty_checker.check(&ident)?;

                    self.resolver.add_type(ident.as_str(), value.clone());

                    nodes.push(Node::Struct(value));
                }
                Rule::Enum => {
                    let (ident, value) = self.generate_enum(p)?;

                    ty_checker.check(&ident)?;

                    self.resolver.add_type(ident.as_str(), value.clone());

                    nodes.push(Node::Enum(value));
                }
                Rule::Interface => {
                    let (ident, value) = self.generate_interface(p)?;

                    if_checker.check(&ident)?;

                    nodes.push(Node::Interface(value));
                }
                Rule::EOI => {}
                _ => unreachable!("unexpected token {:?}", p),
            }
        }

        Ok(self.lang.generate_defs(Defs::new(uses, nodes))?)
    }

    fn generate_use(&mut self, p: Pair<Rule>) -> Result<Use> {
        trace!("Generating use: {}", p.as_str());

        let path = get_all(&p, Rule::Path);
        let ns = path.clone()
            .into_iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join("::");
        let path = path.into_iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join("/");
        let path = format!("{}.rpc", path);
        let fullpath = format!("{}/{}", self.loader.current_dir(), path);

        self.load_submodule(&path, &ns)
            .chain_err(|| error(&path))
            .chain_err(|| load_error(&p, &fullpath))?;

        Ok(self.lang.generate_use(Use::new(&ns, &path))?)
    }

    fn generate_struct<'a>(&mut self, p: Pair<'a, Rule>) -> Result<(Pair<'a, Rule>, Struct)> {
        trace!("Generating struct:\n {}", p.as_str());

        let mut checker = DupChecker::new("struct member name");

        let mut fields = Vec::new();

        for f in get_all(&p, Rule::Field) {
            let comment = get_comment(&f);
            let ident = get(&f, Rule::Identifier);
            let gty = get(&f, Rule::GenericType);
            let value = parse_value(&f)?;

            checker.check(&ident)?;

            fields.push(self.lang.generate_field(Field::new(
                comment,
                ident.as_str(),
                self.resolver.resolve_generic_type(&gty)?,
                value,
            ))?);
        }

        let comment = get_comment(&p);
        let ident = get(&p, Rule::Identifier);

        Ok((
            ident.clone(),
            self.lang
                .generate_struct(Struct::new(comment, ident.as_str(), fields))?,
        ))
    }

    fn generate_enum<'a>(&mut self, p: Pair<'a, Rule>) -> Result<(Pair<'a, Rule>, Enum)> {
        trace!("Generating enum:\n {}", p.as_str());

        let mut checker = DupChecker::new("enum variant name");

        let mut variants = Vec::new();

        let uty = get(&p, Rule::Type);

        for f in get_all(&p, Rule::Variant) {
            let comment = get_comment(&f);
            let ident = get(&f, Rule::Identifier);
            let value = parse_value(&f)?;

            checker.check(&ident)?;

            variants.push(self.lang.generate_variant(Variant::new(
                comment,
                ident.as_str(),
                self.resolver.resolve_type(&uty)?,
                value,
            ))?);
        }

        let comment = get_comment(&p);
        let ident = get(&p, Rule::Identifier);

        Ok((
            ident.clone(),
            self.lang.generate_enum(Enum::new(
                comment,
                ident.as_str(),
                self.resolver.resolve_type(&uty)?,
                variants,
            ))?,
        ))
    }

    fn generate_interface<'a>(&mut self, p: Pair<'a, Rule>) -> Result<(Pair<'a, Rule>, Interface)> {
        trace!("Generating interface:\n {}", p.as_str());

        let mut funcs = Vec::new();

        let mut checker = DupChecker::new("function name");

        for f in get_all(&p, Rule::Function) {
            let comment = get_comment(&p);
            let ident = get(&f, Rule::Identifier);
            let mut args = Vec::new();

            checker.check(&ident)?;

            let mut arg_checker = DupChecker::new("argument name");

            for a in get_all(&f, Rule::Argument) {
                let ident = get(&a, Rule::Identifier);
                let ty = get(&a, Rule::Type);

                arg_checker.check(&ident)?;

                args.push(self.lang
                    .generate_arg(Arg::new(ident.as_str(), self.resolver.resolve_type(&ty)?))?);
            }

            let r = get_opt(&f, Rule::ReturnType);
            let r = if let Some(r) = r {
                let mut rs = Vec::new();

                for ty in get_all(&r, Rule::Type) {
                    rs.push(self.resolver.resolve_type(&ty)?);
                }

                Some(rs)
            } else {
                None
            };

            funcs.push(self.lang.generate_func(Func::new(
                comment,
                ident.as_str(),
                args,
                r.unwrap_or(Vec::new()),
            ))?);
        }

        let comment = get_comment(&p);
        let ident = get(&p, Rule::Identifier);
        let pattern = get(&p, Rule::Pattern).as_str();

        Ok((
            ident.clone(),
            self.lang
                .generate_interface(Interface::new(comment, ident.as_str(), &pattern, funcs))?,
        ))
    }
}

pub fn compile(path: &str) -> Result<Value> {
    let mut nullgen = NullGenerator;
    let mut gen = Generator::new(&mut nullgen);

    let cwd = std::env::current_dir().map_err(|e| file_error(e))?;
    let fullpath = format!("{}/{}", cwd.to_string_lossy(), path);

    let model = gen.generate(path).chain_err(|| error(&fullpath))?;

    serde_json::to_value(model).map_err(|e| pack_error(e))
}
