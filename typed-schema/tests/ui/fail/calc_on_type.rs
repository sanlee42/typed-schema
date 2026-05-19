use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[shape(name = "bad", calc = "1 + 1")]
struct Bad {
    id: String,
}

fn main() {}
