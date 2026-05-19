use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[shape(name = "input", desc = "Input")]
struct Input {
    id: String,
}

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[serde(rename_all = "camelCase")]
#[shape(name = "output")]
struct Output {
    output_id: String,
    #[shape(
        calc = "base + bonus",
        grain = "user",
        source = "mart.user_score"
    )]
    score: i64,
}

#[derive(typed_schema::Op)]
#[op(name = "output.get", input = Input, output = Output, http(GET, "/output"), desc = "Get output")]
struct OutputGet;

fn main() {
    let index = typed_schema::index! {
        types: [Input, Output],
        ops: [OutputGet],
    };
    typed_schema::check(&index).unwrap();
}
