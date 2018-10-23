use serde_json::value::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Type {
    Primitive(Primitive),
    Struct(Struct),
    Enum(Enum),
    Template(Template),
}

impl From<Primitive> for Type {
    fn from(p: Primitive) -> Type {
        Type::Primitive(p)
    }
}

impl From<Struct> for Type {
    fn from(p: Struct) -> Type {
        Type::Struct(p)
    }
}

impl From<Enum> for Type {
    fn from(p: Enum) -> Type {
        Type::Enum(p)
    }
}

impl From<Template> for Type {
    fn from(p: Template) -> Type {
        Type::Template(p)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trait {
    Bool,
    Integer,
    Float,
    String,
    Struct,
    Enum,
    Template,
    Interface,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Primitive {
    pub name: String,
    #[serde(rename = "trait")]
    pub tt: Trait,
    #[serde(flatten)]
    pub custom: Value,
}

impl Primitive {
    pub fn new(name: &str, tt: Trait) -> Self {
        Self {
            name: name.into(),
            tt,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Struct {
    pub comment: Option<String>,
    pub name: String,
    #[serde(rename = "trait")]
    pub tt: Trait,
    pub members: Vec<Field>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Struct {
    pub fn new(comment: Option<&str>, name: &str, members: Vec<Field>) -> Self {
        Self {
            comment: comment.map(|s| s.into()),
            name: name.into(),
            tt: Trait::Struct,
            members,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub comment: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Type,
    pub value: Option<Value>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Field {
    pub fn new(comment: Option<&str>, name: &str, ty: Type, value: Option<Value>) -> Self {
        Self {
            comment: comment.map(|s| s.into()),
            name: name.into(),
            ty,
            value,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enum {
    pub comment: Option<String>,
    pub name: String,
    pub utype: Box<Type>,
    #[serde(rename = "trait")]
    pub tt: Trait,
    pub members: Vec<Variant>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Enum {
    pub fn new(comment: Option<&str>, name: &str, utype: Type, members: Vec<Variant>) -> Self {
        Self {
            comment: comment.map(|s| s.into()),
            name: name.into(),
            utype: Box::new(utype),
            tt: Trait::Enum,
            members,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub comment: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Type,
    pub value: Option<Value>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Variant {
    pub fn new(comment: Option<&str>, name: &str, ty: Type, value: Option<Value>) -> Self {
        Self {
            comment: comment.map(|s| s.into()),
            name: name.into(),
            ty,
            value,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub comment: Option<String>,
    pub name: String,
    pub pattern: String,
    #[serde(rename = "trait")]
    pub tt: Trait,
    pub funcs: Vec<Func>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Interface {
    pub fn new(comment: Option<&str>, name: &str, pattern: &str, funcs: Vec<Func>) -> Self {
        Self {
            comment: comment.map(|s| s.into()),
            name: name.into(),
            pattern: pattern.into(),
            tt: Trait::Interface,
            funcs,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Func {
    pub comment: Option<String>,
    pub name: String,
    pub args: Vec<Arg>,
    pub ret: Vec<Type>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Func {
    pub fn new(comment: Option<&str>, name: &str, args: Vec<Arg>, ret: Vec<Type>) -> Self {
        Self {
            comment: comment.map(|s| s.into()),
            name: name.into(),
            args,
            ret,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arg {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Type,
    #[serde(flatten)]
    pub custom: Value,
}

impl Arg {
    pub fn new(name: &str, ty: Type) -> Self {
        Self {
            name: name.into(),
            ty,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    #[serde(rename = "trait")]
    pub tt: Trait,
    pub params: Vec<Type>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Template {
    pub fn new(name: &str, params: Vec<Type>) -> Self {
        Self {
            name: name.into(),
            tt: Trait::Template,
            params,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defs {
    pub uses: Vec<Use>,
    pub nodes: Vec<Node>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Defs {
    pub fn new(uses: Vec<Use>, nodes: Vec<Node>) -> Self {
        Self {
            uses,
            nodes,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Use {
    pub namespace: String,
    pub path: String,
    #[serde(flatten)]
    pub custom: Value,
}

impl Use {
    pub fn new(namespace: &str, path: &str) -> Self {
        Self {
            namespace: namespace.into(),
            path: path.into(),
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Node {
    Struct(Struct),
    Enum(Enum),
    Interface(Interface),
}
