use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
struct MissingName {
    id: String,
}

fn main() {}
