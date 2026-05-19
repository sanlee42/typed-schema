use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[shape(name = "bad")]
struct Bad {
    #[shape(grain = "user")]
    score: i64,
}

fn main() {}
