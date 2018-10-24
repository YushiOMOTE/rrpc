use pest::iterators::{Pair, Pairs};

use std::collections::HashSet;

use super::error;
use super::parser::{get, get_all, get_comment, get_opt, parse, parse_value, Rule};
use super::resolver::Resolver;
use super::error::{Result, ResultExt};
use super::loader::Loader;
use super::types::*;

use super::lang::LangGenerator;

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
            Err(error::duplicated(self.name, p))
        }
    }
}

fn primitive(resolver: &mut Resolver, lang: &mut LangGenerator, ident: &str, tt: Trait) {
    resolver.add_type(
        ident.into(),
        lang.generate_primitive(Primitive::new(ident, tt)),
    );
}

pub struct Generator<'g> {
    resolver: Resolver,
    loader: Loader,
    lang: &'g mut LangGenerator,
}

impl<'g> Generator<'g> {
    pub fn new(lang: &'g mut LangGenerator) -> Self {
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

    pub fn generate(&mut self, path: &str) -> Result<Defs> {
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

        let mut checker = DupChecker::new("type name");

        for p in pairs {
            match p.as_rule() {
                Rule::Use => {
                    uses.push(self.generate_use(p)?);
                }
                Rule::Struct => {
                    let (ident, value) = self.generate_struct(p)?;

                    checker.check(&ident)?;

                    self.resolver.add_type(ident.as_str(), value.clone());

                    nodes.push(Node::Struct(value));
                }
                Rule::Enum => {
                    let (ident, value) = self.generate_enum(p)?;

                    checker.check(&ident)?;

                    self.resolver.add_type(ident.as_str(), value.clone());

                    nodes.push(Node::Enum(value));
                }
                Rule::Interface => {
                    let (ident, value) = self.generate_interface(p)?;

                    checker.check(&ident)?;

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
            .chain_err(|| error::error(&path))
            .chain_err(|| error::load_error(&p, &fullpath))?;

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
