extern crate self as typed_schema;

use std::collections::BTreeSet;
use std::fmt;

pub use schemars;
pub use typed_schema_macros::{Op, Shape};

pub const VERSION: &str = "1";

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Index {
    pub version: String,
    pub types: Vec<Type>,
    pub ops: Vec<Op>,
}

impl Index {
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: VERSION.to_owned(),
            types: Vec::new(),
            ops: Vec::new(),
        }
    }

    pub fn sort(&mut self) {
        self.types.sort_by(|left, right| left.name.cmp(&right.name));
        self.ops.sort_by(|left, right| left.name.cmp(&right.name));
    }
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Type {
    pub name: String,
    pub rust: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    pub schema: serde_json::Value,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub name: String,
    pub ident: String,
    pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<Metric>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Metric {
    pub calc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grain: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Op {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    pub input: String,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<Http>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Http {
    pub method: Method,
    pub path: String,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

pub trait Shape: schemars::JsonSchema {
    fn shape() -> Type;
    fn name() -> &'static str;
}

pub trait OpSpec {
    fn op() -> Op;
}

pub mod tool {
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};

    use crate::{Shape, VERSION};

    #[derive(
        Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema, typed_schema::Shape,
    )]
    #[shape(name = "tool_manifest")]
    pub struct Manifest {
        pub version: String,
        pub tools: Vec<Tool>,
        #[serde(default, skip_serializing_if = "Value::is_null")]
        pub refs: Value,
    }

    impl Manifest {
        #[must_use]
        pub fn new(tools: Vec<Tool>) -> Self {
            Self {
                version: VERSION.to_owned(),
                tools,
                refs: Value::Null,
            }
        }

        #[must_use]
        pub fn refs(mut self, refs: Value) -> Self {
            self.refs = refs;
            self
        }

        #[must_use]
        pub fn openai_tools(&self) -> Vec<Value> {
            self.tools.iter().map(Tool::openai).collect()
        }
    }

    #[derive(
        Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema, typed_schema::Shape,
    )]
    #[shape(name = "tool_def")]
    pub struct Tool {
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub desc: Option<String>,
        pub input: String,
        pub schema: Value,
    }

    impl Tool {
        #[must_use]
        pub fn for_shape<T>(name: impl Into<String>, desc: impl Into<String>) -> Self
        where
            T: Shape,
        {
            let shape = T::shape();
            Self {
                name: name.into(),
                desc: Some(desc.into()),
                input: shape.name,
                schema: tool_schema(shape.schema),
            }
        }

        #[must_use]
        pub fn openai(&self) -> Value {
            let mut function = json!({
                "name": self.name,
                "parameters": self.schema,
            });
            if let Some(desc) = &self.desc {
                function["description"] = Value::String(desc.clone());
            }

            json!({
                "type": "function",
                "function": function,
            })
        }
    }

    #[derive(
        Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema, typed_schema::Shape,
    )]
    #[serde(deny_unknown_fields)]
    #[shape(name = "tool_draft")]
    pub struct Draft {
        #[serde(default)]
        pub calls: Vec<Call>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub action: Option<Action>,
    }

    #[derive(
        Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema, typed_schema::Shape,
    )]
    #[serde(deny_unknown_fields)]
    #[shape(name = "tool_call")]
    pub struct Call {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub id: Option<String>,
        pub tool: String,
        #[serde(default)]
        pub args: Value,
    }

    #[derive(
        Clone,
        Debug,
        PartialEq,
        Eq,
        Serialize,
        Deserialize,
        schemars::JsonSchema,
        typed_schema::Shape,
    )]
    #[serde(tag = "kind", rename_all = "snake_case")]
    #[shape(name = "tool_action")]
    pub enum Action {
        Clarify {
            #[serde(default, skip_serializing_if = "Option::is_none")]
            field: Option<String>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            question: Option<String>,
        },
        Unsupported {
            #[serde(default, skip_serializing_if = "Option::is_none")]
            field: Option<String>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            reason: Option<String>,
        },
    }

    fn tool_schema(mut schema: Value) -> Value {
        if let Value::Object(map) = &mut schema {
            map.remove("$schema");
        }
        schema
    }
}

#[must_use]
pub fn json_schema<T: schemars::JsonSchema>() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("schemars schema should serialize")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    EmptyTypeName,
    EmptyOpName,
    EmptyFieldName {
        ty: String,
    },
    EmptyHttpPath {
        op: String,
    },
    EmptyMetricAttr {
        ty: String,
        field: String,
        attr: &'static str,
    },
    DuplicateType(String),
    DuplicateOp(String),
    DuplicateField {
        ty: String,
        field: String,
    },
    MissingInput {
        op: String,
        ty: String,
    },
    MissingOutput {
        op: String,
        ty: String,
    },
    MissingSchemaProperties {
        ty: String,
    },
    MissingSchemaField {
        ty: String,
        field: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTypeName => write!(f, "empty type name"),
            Self::EmptyOpName => write!(f, "empty op name"),
            Self::EmptyFieldName { ty } => write!(f, "type {ty} has an empty field name"),
            Self::EmptyHttpPath { op } => write!(f, "op {op} has an empty HTTP path"),
            Self::EmptyMetricAttr { ty, field, attr } => {
                write!(f, "type {ty} field {field} has an empty metric {attr}")
            }
            Self::DuplicateType(name) => write!(f, "duplicate type: {name}"),
            Self::DuplicateOp(name) => write!(f, "duplicate op: {name}"),
            Self::DuplicateField { ty, field } => {
                write!(f, "type {ty} has duplicate field {field}")
            }
            Self::MissingInput { op, ty } => {
                write!(f, "op {op} references missing input type {ty}")
            }
            Self::MissingOutput { op, ty } => {
                write!(f, "op {op} references missing output type {ty}")
            }
            Self::MissingSchemaProperties { ty } => {
                write!(
                    f,
                    "type {ty} has fields but its JSON Schema has no properties"
                )
            }
            Self::MissingSchemaField { ty, field } => {
                write!(
                    f,
                    "type {ty} field {field} is missing from JSON Schema properties"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

pub fn check(index: &Index) -> Result<(), Error> {
    let mut types = BTreeSet::new();
    for ty in &index.types {
        if ty.name.trim().is_empty() {
            return Err(Error::EmptyTypeName);
        }
        if !types.insert(ty.name.as_str()) {
            return Err(Error::DuplicateType(ty.name.clone()));
        }
        check_fields(ty)?;
    }

    let mut ops = BTreeSet::new();
    for op in &index.ops {
        if op.name.trim().is_empty() {
            return Err(Error::EmptyOpName);
        }
        if !ops.insert(op.name.as_str()) {
            return Err(Error::DuplicateOp(op.name.clone()));
        }
        if let Some(http) = &op.http
            && http.path.trim().is_empty()
        {
            return Err(Error::EmptyHttpPath {
                op: op.name.clone(),
            });
        }
        if !types.contains(op.input.as_str()) {
            return Err(Error::MissingInput {
                op: op.name.clone(),
                ty: op.input.clone(),
            });
        }
        if !types.contains(op.output.as_str()) {
            return Err(Error::MissingOutput {
                op: op.name.clone(),
                ty: op.output.clone(),
            });
        }
    }

    Ok(())
}

fn check_fields(ty: &Type) -> Result<(), Error> {
    let mut fields = BTreeSet::new();
    for field in &ty.fields {
        if field.name.trim().is_empty() {
            return Err(Error::EmptyFieldName {
                ty: ty.name.clone(),
            });
        }
        if !fields.insert(field.name.as_str()) {
            return Err(Error::DuplicateField {
                ty: ty.name.clone(),
                field: field.name.clone(),
            });
        }
        if let Some(metric) = &field.metric {
            check_metric_attr(ty, field, "calc", &metric.calc)?;
            if let Some(grain) = &metric.grain {
                check_metric_attr(ty, field, "grain", grain)?;
            }
            for source in &metric.sources {
                check_metric_attr(ty, field, "source", source)?;
            }
        }
    }

    if ty.fields.is_empty() {
        return Ok(());
    }

    let Some(properties) = ty
        .schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
    else {
        return Err(Error::MissingSchemaProperties {
            ty: ty.name.clone(),
        });
    };
    for field in &ty.fields {
        if !properties.contains_key(&field.name) {
            return Err(Error::MissingSchemaField {
                ty: ty.name.clone(),
                field: field.name.clone(),
            });
        }
    }

    Ok(())
}

fn check_metric_attr(
    ty: &Type,
    field: &Field,
    attr: &'static str,
    value: &str,
) -> Result<(), Error> {
    if value.trim().is_empty() {
        return Err(Error::EmptyMetricAttr {
            ty: ty.name.clone(),
            field: field.name.clone(),
            attr,
        });
    }
    Ok(())
}

#[macro_export]
macro_rules! index {
    (types: [$($ty:ty),* $(,)?], ops: [$($op:ty),* $(,)?] $(,)?) => {{
        let mut index = $crate::Index::new();
        $(
            index.types.push(<$ty as $crate::Shape>::shape());
        )*
        $(
            index.ops.push(<$op as $crate::OpSpec>::op());
        )*
        index.sort();
        if let ::std::result::Result::Err(err) = $crate::check(&index) {
            ::std::panic!("invalid typed schema index: {}", err);
        }
        index
    }};
}
