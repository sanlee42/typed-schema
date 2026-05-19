use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
struct Extra {
    id: String,
}

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[shape(name = "bad")]
struct Bad {
    #[serde(flatten)]
    #[shape(calc = "extra.id")]
    extra: Extra,
}

fn main() {}
