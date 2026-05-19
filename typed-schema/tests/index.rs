use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use typed_schema::{Error, Http, Index, Method, Metric, Op, Type, check};

mod names {
    pub const RAW_USER_SCORE: &str = "raw.user_score";
    pub const MART_USER_SCORE: &str = "mart.user_score";
    pub const USER: &str = "user";
}

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[shape(name = "get_user", desc = "Get user input")]
struct GetUser {
    id: String,
}

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[shape(name = "user", desc = "User output")]
struct User {
    id: String,
    #[shape(calc = "base + bonus", grain = "user", source = "mart.user_score")]
    score: i64,
}

#[derive(typed_schema::Op)]
#[op(
    name = "user.get",
    input = GetUser,
    output = User,
    http(GET, "/users/{id}"),
    desc = "Get one user"
)]
struct GetUserOp;

#[derive(Serialize, Deserialize, JsonSchema, typed_schema::Shape)]
#[serde(rename_all = "camelCase")]
#[shape(name = "wire_user")]
#[allow(dead_code)]
struct WireUser {
    user_id: String,
    #[serde(rename = "kind")]
    user_kind: String,
    #[serde(rename = "scoreValue")]
    #[shape(
        name = "scoreValue",
        calc = "base + bonus",
        grain = names::USER,
        source = names::RAW_USER_SCORE,
        source = names::MART_USER_SCORE
    )]
    score_value: i64,
    #[serde(skip)]
    skipped: Option<String>,
    #[serde(skip_serializing)]
    internal_score: Option<i64>,
    #[serde(flatten)]
    extra: WireExtra,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct WireExtra {
    extra_id: String,
}

#[test]
fn index_macro_builds_sorted_schema() {
    let index = typed_schema::index! {
        types: [User, GetUser],
        ops: [GetUserOp],
    };

    assert_eq!(index.version, typed_schema::VERSION);
    assert_eq!(
        index
            .types
            .iter()
            .map(|ty| ty.name.as_str())
            .collect::<Vec<_>>(),
        vec!["get_user", "user"]
    );
    assert_eq!(index.ops[0].name, "user.get");
    assert_eq!(index.ops[0].input, "get_user");
    assert_eq!(index.ops[0].output, "user");
    assert_eq!(
        index.ops[0].http,
        Some(Http {
            method: Method::Get,
            path: "/users/{id}".to_owned(),
        })
    );
    check(&index).unwrap();
}

#[test]
fn field_metric_is_kept() {
    let ty = <User as typed_schema::Shape>::shape();
    let score = ty
        .fields
        .iter()
        .find(|field| field.name == "score")
        .unwrap();

    assert_eq!(
        score.metric.as_ref(),
        Some(&Metric {
            calc: "base + bonus".to_owned(),
            grain: Some("user".to_owned()),
            sources: vec!["mart.user_score".to_owned()],
        })
    );
    assert_eq!(score.ident, "score");
    assert_eq!(score.ty, "i64");
    assert!(ty.schema.is_object());
}

#[test]
fn field_metric_keeps_multiple_sources() {
    let ty = <WireUser as typed_schema::Shape>::shape();
    let score = ty
        .fields
        .iter()
        .find(|field| field.name == "scoreValue")
        .unwrap();
    let metric = score.metric.as_ref().unwrap();

    assert_eq!(metric.grain.as_deref(), Some("user"));
    assert_eq!(
        metric.sources,
        vec!["raw.user_score".to_owned(), "mart.user_score".to_owned()]
    );
}

#[test]
fn serde_wire_names_are_used_for_fields() {
    let ty = <WireUser as typed_schema::Shape>::shape();
    let fields = ty
        .fields
        .iter()
        .map(|field| (field.name.as_str(), field.ident.as_str()))
        .collect::<Vec<_>>();

    assert_eq!(
        fields,
        vec![
            ("userId", "user_id"),
            ("kind", "user_kind"),
            ("scoreValue", "score_value"),
        ]
    );
    assert!(
        ty.fields
            .iter()
            .find(|field| field.name == "kind")
            .unwrap()
            .metric
            .is_none()
    );
    check(&Index {
        version: typed_schema::VERSION.to_owned(),
        types: vec![ty],
        ops: Vec::new(),
    })
    .unwrap();
}

#[test]
#[should_panic(
    expected = "invalid typed schema index: op user.get references missing input type get_user"
)]
fn index_macro_checks_output() {
    let _ = typed_schema::index! {
        types: [User],
        ops: [GetUserOp],
    };
}

#[test]
fn check_rejects_duplicate_types() {
    let ty = bare_type("user");
    let index = Index {
        version: typed_schema::VERSION.to_owned(),
        types: vec![ty.clone(), ty],
        ops: Vec::new(),
    };

    assert_eq!(check(&index), Err(Error::DuplicateType("user".to_owned())));
}

#[test]
fn check_rejects_empty_type_name() {
    let index = Index {
        version: typed_schema::VERSION.to_owned(),
        types: vec![bare_type("")],
        ops: Vec::new(),
    };

    assert_eq!(check(&index), Err(Error::EmptyTypeName));
}

#[test]
fn check_rejects_empty_op_name() {
    let index = Index {
        version: typed_schema::VERSION.to_owned(),
        types: vec![bare_type("input"), bare_type("output")],
        ops: vec![bare_op("", "input", "output")],
    };

    assert_eq!(check(&index), Err(Error::EmptyOpName));
}

#[test]
fn check_rejects_empty_http_path() {
    let mut op = bare_op("user.get", "input", "output");
    op.http = Some(Http {
        method: Method::Get,
        path: " ".to_owned(),
    });
    let index = Index {
        version: typed_schema::VERSION.to_owned(),
        types: vec![bare_type("input"), bare_type("output")],
        ops: vec![op],
    };

    assert_eq!(
        check(&index),
        Err(Error::EmptyHttpPath {
            op: "user.get".to_owned(),
        })
    );
}

#[test]
fn check_rejects_bad_fields() {
    let cases = [
        (
            type_with_fields(vec![bare_field("field"), bare_field("field")]),
            Error::DuplicateField {
                ty: "user".to_owned(),
                field: "field".to_owned(),
            },
        ),
        (
            type_with_fields(vec![bare_field("")]),
            Error::EmptyFieldName {
                ty: "user".to_owned(),
            },
        ),
        (
            type_with_fields(vec![field_with_metric(
                "field",
                Metric {
                    calc: " ".to_owned(),
                    ..metric()
                },
            )]),
            Error::EmptyMetricAttr {
                ty: "user".to_owned(),
                field: "field".to_owned(),
                attr: "calc",
            },
        ),
    ];

    for (ty, err) in cases {
        let index = Index {
            version: typed_schema::VERSION.to_owned(),
            types: vec![ty],
            ops: Vec::new(),
        };
        assert_eq!(check(&index), Err(err));
    }
}

#[test]
fn check_rejects_schema_mismatch() {
    let missing_properties =
        type_with_schema_and_fields(serde_json::json!({}), vec![bare_field("field")]);
    let missing_field = type_with_schema_and_fields(
        serde_json::json!({"type": "object", "properties": {"other": {"type": "string"}}}),
        vec![bare_field("field")],
    );

    assert_eq!(
        check(&Index {
            version: typed_schema::VERSION.to_owned(),
            types: vec![missing_properties],
            ops: Vec::new(),
        }),
        Err(Error::MissingSchemaProperties {
            ty: "user".to_owned(),
        })
    );
    assert_eq!(
        check(&Index {
            version: typed_schema::VERSION.to_owned(),
            types: vec![missing_field],
            ops: Vec::new(),
        }),
        Err(Error::MissingSchemaField {
            ty: "user".to_owned(),
            field: "field".to_owned(),
        })
    );
}

#[test]
fn check_rejects_duplicate_ops() {
    let op = bare_op("user.get", "input", "output");
    let index = Index {
        version: typed_schema::VERSION.to_owned(),
        types: vec![bare_type("input"), bare_type("output")],
        ops: vec![op.clone(), op],
    };

    assert_eq!(
        check(&index),
        Err(Error::DuplicateOp("user.get".to_owned()))
    );
}

#[test]
fn check_rejects_missing_input() {
    let index = Index {
        version: typed_schema::VERSION.to_owned(),
        types: vec![bare_type("output")],
        ops: vec![bare_op("user.get", "input", "output")],
    };

    assert_eq!(
        check(&index),
        Err(Error::MissingInput {
            op: "user.get".to_owned(),
            ty: "input".to_owned(),
        })
    );
}

#[test]
fn check_rejects_missing_output() {
    let index = Index {
        version: typed_schema::VERSION.to_owned(),
        types: vec![bare_type("input")],
        ops: vec![bare_op("user.get", "input", "output")],
    };

    assert_eq!(
        check(&index),
        Err(Error::MissingOutput {
            op: "user.get".to_owned(),
            ty: "output".to_owned(),
        })
    );
}

fn bare_type(name: &str) -> Type {
    Type {
        name: name.to_owned(),
        rust: name.to_owned(),
        desc: None,
        schema: serde_json::json!({}),
        fields: Vec::new(),
    }
}

fn type_with_fields(fields: Vec<typed_schema::Field>) -> Type {
    let properties = fields
        .iter()
        .filter(|field| !field.name.trim().is_empty())
        .map(|field| (field.name.clone(), serde_json::json!({"type": "string"})))
        .collect::<serde_json::Map<_, _>>();
    type_with_schema_and_fields(
        serde_json::Value::Object(
            [
                (
                    "type".to_owned(),
                    serde_json::Value::String("object".to_owned()),
                ),
                (
                    "properties".to_owned(),
                    serde_json::Value::Object(properties),
                ),
            ]
            .into_iter()
            .collect(),
        ),
        fields,
    )
}

fn type_with_schema_and_fields(
    schema: serde_json::Value,
    fields: Vec<typed_schema::Field>,
) -> Type {
    Type {
        name: "user".to_owned(),
        rust: "User".to_owned(),
        desc: None,
        schema,
        fields,
    }
}

fn bare_field(name: &str) -> typed_schema::Field {
    typed_schema::Field {
        name: name.to_owned(),
        ident: name.to_owned(),
        ty: "String".to_owned(),
        metric: None,
    }
}

fn field_with_metric(name: &str, metric: Metric) -> typed_schema::Field {
    typed_schema::Field {
        metric: Some(metric),
        ..bare_field(name)
    }
}

fn metric() -> Metric {
    Metric {
        calc: "base + bonus".to_owned(),
        grain: Some("user".to_owned()),
        sources: vec!["mart.user_score".to_owned()],
    }
}

fn bare_op(name: &str, input: &str, output: &str) -> Op {
    Op {
        name: name.to_owned(),
        desc: None,
        input: input.to_owned(),
        output: output.to_owned(),
        http: None,
    }
}
