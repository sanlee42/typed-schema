use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[shape(name = "bad")]
struct Bad {
    #[serde(skip)]
    #[shape(calc = "hidden")]
    hidden: String,
}

fn main() {}
