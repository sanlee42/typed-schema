use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
struct Input {
    id: String,
}

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[shape(name = "output")]
struct Output {
    id: String,
}

#[derive(typed_schema::Op)]
#[op(name = "bad.op", input = Input, output = Output)]
struct BadOp;

fn main() {
    let _ = <BadOp as typed_schema::OpSpec>::op();
}
