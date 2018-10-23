use serde_json::value::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Type {
    Primitive(Primitive),
    Struct(Struct),
    Enum(Enum),
    Template(Template),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Trait {
    Bool,
    Integer,
    Float,
    String,
    Struct,
    Enum,
    Template,
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
    pub comment: String,
    pub name: String,
    #[serde(rename = "trait")]
    pub tt: Trait,
    pub members: Vec<Field>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Struct {
    pub fn new(comment: &str, name: &str, tt: Trait, members: Vec<Field>) -> Self {
        Self {
            comment: comment.into(),
            name: name.into(),
            tt,
            members,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub comment: String,
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Type,
    pub value: Option<Value>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Field {
    pub fn new(comment: &str, name: &str, ty: Type, value: Option<Value>) -> Self {
        Self {
            comment: comment.into(),
            name: name.into(),
            ty,
            value,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enum {
    pub comment: String,
    pub name: String,
    pub utype: Box<Type>,
    #[serde(rename = "trait")]
    pub tt: Trait,
    pub members: Vec<Variant>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Enum {
    pub fn new(comment: &str, name: &str, utype: Type, tt: Trait, members: Vec<Variant>) -> Self {
        Self {
            comment: comment.into(),
            name: name.into(),
            utype: Box::new(utype),
            tt,
            members,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub comment: String,
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Type,
    pub value: Option<Value>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Variant {
    pub fn new(comment: &str, name: &str, ty: Type, value: Option<Value>) -> Self {
        Self {
            comment: comment.into(),
            name: name.into(),
            ty,
            value,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub comment: String,
    pub name: String,
    pub pattern: String,
    #[serde(rename = "trait")]
    pub tt: Trait,
    pub funcs: Vec<Func>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Interface {
    pub fn new(comment: &str, name: &str, pattern: &str, tt: Trait, funcs: Vec<Func>) -> Self {
        Self {
            comment: comment.into(),
            name: name.into(),
            pattern: pattern.into(),
            tt,
            funcs,
            custom: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Func {
    pub comment: String,
    pub name: String,
    pub args: Vec<Arg>,
    pub ret: Vec<Type>,
    #[serde(flatten)]
    pub custom: Value,
}

impl Func {
    pub fn new(comment: &str, name: &str, args: Vec<Arg>, ret: Vec<Type>) -> Self {
        Self {
            comment: comment.into(),
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
    pub fn new(name: &str, tt: Trait, params: Vec<Type>) -> Self {
        Self {
            name: name.into(),
            tt,
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
    pub fn new(namespace: String, path: String) -> Self {
        Self {
            namespace,
            path,
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
