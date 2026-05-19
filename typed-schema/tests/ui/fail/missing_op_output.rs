use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[shape(name = "input")]
struct Input {
    id: String,
}

#[derive(typed_schema::Op)]
#[op(name = "bad.op", input = Input)]
struct BadOp;

fn main() {}
