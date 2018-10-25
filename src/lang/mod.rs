use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::Result;
use crate::types::*;
use crate::error;

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

mod null;

pub use self::null::NullGenerator;

type Gen = Arc<Mutex<LangGenerator + Send + Sync>>;
type GenTable = HashMap<String, Gen>;

lazy_static! {
    pub static ref LANG_GENERATORS: Arc<Mutex<GenTable>> = {
        let mut map = GenTable::new();

        map.insert("null".into(), Arc::new(Mutex::new(NullGenerator)));

        Arc::new(Mutex::new(map))
    };
}

pub fn register_generator<T: LangGenerator + Send + Sync + 'static>(key: &str, gen: T) {
    LANG_GENERATORS
        .lock()
        .unwrap()
        .insert(key.into(), Arc::new(Mutex::new(gen)));
}

pub fn get_generator(key: &str) -> Result<Gen> {
    LANG_GENERATORS
        .lock()
        .unwrap()
        .get(key)
        .map(|v| v.clone())
        .ok_or(error::generator_not_found(key))
}
