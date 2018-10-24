use pest::error::{Error as PestError, ErrorVariant};
use pest::iterators::Pair;

use super::parser::Rule;

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

pub fn error(path: &str) -> Error {
    ErrorKind::Error(path.to_string()).into()
}

pub fn file_error<T: ToString>(e: T) -> Error {
    ErrorKind::FileError(e.to_string()).into()
}

pub fn value_error<T: ToString>(p: &Pair<Rule>, e: T) -> Error {
    ErrorKind::ValueError(PestError::new_from_span(
        ErrorVariant::CustomError {
            message: format!("value needs to be a valid json: {}", e.to_string()),
        },
        p.as_span(),
    )).into()
}

pub fn type_not_found(p: &Pair<Rule>) -> Error {
    ErrorKind::TypeNotFound(PestError::new_from_span(
        ErrorVariant::CustomError {
            message: format!("type not found: {}", p.as_str()),
        },
        p.as_span(),
    )).into()
}

pub fn load_error(p: &Pair<Rule>, module: &str) -> Error {
    ErrorKind::LoadError(PestError::new_from_span(
        ErrorVariant::CustomError {
            message: format!("couldn't load module: {}", module),
        },
        p.as_span(),
    )).into()
}

pub fn duplicated(name: &str, p: &Pair<Rule>) -> Error {
    ErrorKind::Duplicated(PestError::new_from_span(
        ErrorVariant::CustomError {
            message: format!("duplicated {}: {}", name, p.as_str()),
        },
        p.as_span(),
    )).into()
}

pub fn parse_error(e: PestError<Rule>) -> Error {
    ErrorKind::ParseError(e).into()
}

pub fn pack_error(e: serde_json::error::Error) -> Error {
    ErrorKind::PackError(e).into()
}
