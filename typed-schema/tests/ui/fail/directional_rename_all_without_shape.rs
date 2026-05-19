use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
#[shape(name = "bad")]
struct Bad {
    field_name: String,
}

fn main() {}
