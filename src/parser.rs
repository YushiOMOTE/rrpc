use pest::Parser;
use pest::iterators::{Pair, Pairs};

use serde_json::Value;

use super::error::{self, Result};

#[derive(Parser)]
#[grammar = "rpc.pest"]
struct RpcParser;

pub fn get<'a>(p: &Pair<'a, Rule>, rule: Rule) -> Pair<'a, Rule> {
    match get_opt(p, rule) {
        Some(p) => p,
        None => panic!("missing {:?} in {}", rule, p.as_str()),
    }
}

pub fn get_opt<'a>(p: &Pair<'a, Rule>, rule: Rule) -> Option<Pair<'a, Rule>> {
    for pair in p.clone().into_inner() {
        if rule == pair.as_rule() {
            return Some(pair);
        }
    }
    None
}

pub fn get_all<'a>(p: &Pair<'a, Rule>, rule: Rule) -> Vec<Pair<'a, Rule>> {
    let mut array = Vec::new();
    for pair in p.clone().into_inner() {
        if rule == pair.as_rule() {
            array.push(pair);
        }
    }
    array
}

pub fn get_comment<'a>(p: &'a Pair<Rule>) -> Option<&'a str> {
    let p = get_opt(p, Rule::CommentLine)?;
    get_opt(&p, Rule::Comment).map(|p| p.as_str())
}

pub fn parse<'a>(s: &'a str) -> Result<Pairs<'a, Rule>> {
    RpcParser::parse(Rule::File, s).map_err(|e| error::parse_error(e))
}

pub fn parse_value(p: &Pair<Rule>) -> Result<Option<Value>> {
    match get_opt(p, Rule::Value).map(|p| p.as_str()) {
        Some(s) => Ok(Some(serde_json::from_str(s).map_err(|e| error::value_error(p, e))?)),
        None => Ok(None),
    }
}
