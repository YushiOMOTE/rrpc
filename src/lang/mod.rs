use super::Result;
use super::types::*;

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
