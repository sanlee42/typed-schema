use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[shape(name = "bad")]
struct Bad {
    #[serde(rename(serialize = "fieldOut", deserialize = "field_in"))]
    field_name: String,
}

fn main() {}
