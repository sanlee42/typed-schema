use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[serde(deny_unknown_fields)]
#[shape(name = "order_read")]
struct ReadOrder {
    order_id: String,
}

#[test]
fn tool_is_projected_from_owned_shape() {
    let tool = typed_schema::tool::Tool::for_shape::<ReadOrder>(
        "order_read",
        "Read one order without mutating state.",
    );

    assert_eq!(tool.name, "order_read");
    assert_eq!(tool.input, "order_read");
    assert_eq!(
        tool.desc.as_deref(),
        Some("Read one order without mutating state.")
    );
    assert!(tool.schema.get("$schema").is_none());
    assert_eq!(
        tool.schema["properties"]["order_id"]["type"],
        json!("string")
    );
}

#[test]
fn manifest_projects_openai_function_tools() {
    let manifest =
        typed_schema::tool::Manifest::new(vec![typed_schema::tool::Tool::for_shape::<ReadOrder>(
            "order_read",
            "Read one order without mutating state.",
        )])
        .refs(json!({"owner": "typed-schema-test"}));

    assert_eq!(manifest.version, typed_schema::VERSION);
    assert_eq!(manifest.refs["owner"], json!("typed-schema-test"));

    let tools = manifest.openai_tools();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["type"], json!("function"));
    assert_eq!(tools[0]["function"]["name"], json!("order_read"));
    assert_eq!(
        tools[0]["function"]["description"],
        json!("Read one order without mutating state.")
    );
    assert_eq!(
        tools[0]["function"]["parameters"]["properties"]["order_id"]["type"],
        json!("string")
    );
}

#[test]
fn call_draft_keeps_provider_tool_id() {
    let draft = typed_schema::tool::Draft {
        calls: vec![typed_schema::tool::Call {
            id: Some("call_1".to_owned()),
            tool: "order_read".to_owned(),
            args: json!({"order_id": "abc"}),
        }],
        action: None,
    };

    let value = serde_json::to_value(&draft).unwrap();
    let parsed: typed_schema::tool::Draft = serde_json::from_value(value).unwrap();

    assert_eq!(parsed.calls[0].id.as_deref(), Some("call_1"));
    assert_eq!(parsed.calls[0].tool, "order_read");
    assert_eq!(parsed.calls[0].args["order_id"], json!("abc"));
}

#[test]
fn tool_types_have_schema_index_entries() {
    let index = typed_schema::index! {
        types: [
            typed_schema::tool::Action,
            typed_schema::tool::Call,
            typed_schema::tool::Draft,
            typed_schema::tool::Manifest,
            typed_schema::tool::Tool,
        ],
        ops: [],
    };
    let names = index
        .types
        .iter()
        .map(|ty| ty.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            "tool_action",
            "tool_call",
            "tool_def",
            "tool_draft",
            "tool_manifest",
        ]
    );
}
