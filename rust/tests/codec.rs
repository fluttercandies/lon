//! Ports of the deterministic groups in `dart/test/lon_test.dart`.

use lon::{json_to_lon, lon_to_json, ErrorKind, LonCodec, LonError};
use serde_json::json;

fn expect_format_error<T: std::fmt::Debug>(result: Result<T, LonError>, context: &str) {
    let Err(error) = result else {
        panic!("expected a format error for {context:?}, got {result:?}")
    };
    assert_eq!(error.kind(), ErrorKind::Format, "context: {context:?}");
}

// --------------------------------------------------------------- value API

#[test]
fn encodes_and_decodes_json_compatible_values() {
    let value = json!({"users": [{"id": 1, "name": "Ada"}, {"id": 2, "name": "Bob"}]});
    let codec = LonCodec::new();

    let encoded = codec.encode(&value).unwrap();
    assert_eq!(encoded, "{users:[id name;1 Ada;2 Bob]}");
    assert_eq!(codec.decode(&encoded).unwrap(), value);
}

#[test]
fn decodes_duplicate_keys_with_the_last_value_winning() {
    let value = LonCodec::new().decode("{a:1 a:2}").unwrap();
    assert_eq!(value, json!({"a": 2}));
}

#[test]
fn bounds_expanded_value_graphs() {
    let mut value = json!(0);
    for _ in 0..16 {
        value = json!([value.clone(), value.clone()]);
    }
    let constrained = LonCodec::new().max_expanded_elements(64);
    let error = constrained.encode(&value).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidValue);
}

#[test]
fn bounds_serialized_output() {
    let constrained = LonCodec::new().max_output_code_units(3);
    assert_eq!(constrained.encode(&json!("Ada")).unwrap(), "Ada");
    expect_format_error(constrained.encode(&json!("Alan")), "Alan");
}

// -------------------------------------------------------- pretty rendering

#[test]
fn renders_objects_tables_and_container_arrays_across_lines() {
    let pretty = LonCodec::new().pretty(true);
    assert_eq!(
        pretty
            .json_to_lon(
                r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"},{"id":3,"name":"Lin"}],"meta":{"count":3,"tags":["x","y"]},"empty":{},"scalars":[1,2,3]}"#,
            )
            .unwrap(),
        "{\n  users: [\n    id name;\n    1 Ada;\n    2 Bob;\n    3 Lin\n  ]\n  meta: {\n    count: 3\n    tags: [x,\"y\"]\n  }\n  empty: {}\n  scalars: [1,2,3]\n}",
    );
}

#[test]
fn indents_nested_containers_by_depth() {
    let pretty = LonCodec::new().pretty(true);
    assert_eq!(
        pretty
            .json_to_lon(
                r#"{"outer":{"rows":[{"id":1,"items":[{"sku":"x","qty":2},{"sku":"y","qty":1}]}],"leaf":{"a":{"b":1}}},"mix":[1,{"a":"b"},[{"c":"d"}]]}"#,
            )
            .unwrap(),
        "{\n  outer: {\n    rows: [\n      {\n        id: 1\n        items: [\n          sku qty;\n          x 2;\n          \"y\" 1\n        ]\n      }\n    ]\n    leaf: {\n      a: {\n        b: 1\n      }\n    }\n  }\n  mix: [\n    1,\n    {\n      a: b\n    },\n    [\n      {\n        c: d\n      }\n    ]\n  ]\n}",
    );
}

#[test]
fn keeps_table_cells_inline_even_when_they_hold_containers() {
    let pretty = LonCodec::new().pretty(true);
    assert_eq!(
        pretty
            .json_to_lon(
                r#"{"orders":[{"id":"A1","customer":{"id":1,"name":"Ada"},"items":[{"sku":"X","qty":2},{"sku":"Y","qty":1}]},{"id":"A2","customer":{"id":2,"name":"Bob"},"items":[]}]}"#,
            )
            .unwrap(),
        "{\n  orders: [\n    id customer{id name} items;\n    A1 1 Ada [sku qty;X 2;\"Y\" 1];\n    A2 2 Bob []\n  ]\n}",
    );
}

#[test]
fn scalar_roots_stay_compact_in_pretty_mode() {
    let pretty = LonCodec::new().pretty(true);
    for (source, expected) in [
        ("42", "42"),
        (r#""Ada""#, "Ada"),
        ("[1,2,3]", "[1,2,3]"),
        (r#"["a","b"]"#, "[a b]"),
        ("{}", "{}"),
        ("[]", "[]"),
    ] {
        assert_eq!(pretty.json_to_lon(source).unwrap(), expected);
    }
}

#[test]
fn pretty_decodes_to_the_same_values_as_the_compact_form() {
    let pretty = LonCodec::new().pretty(true);
    let source = r#"{"users":[{"id":1,"name":"Ada","meta":{"x":true,"y":null}},{"id":2,"name":"Bob","meta":{"x":false,"y":1}}],"mix":[1,{"a":"b"},[{"c":"d"}]]}"#;
    let compact = json_to_lon(source).unwrap();
    let rendered = pretty.json_to_lon(source).unwrap();

    assert_eq!(
        lon_to_json(&rendered).unwrap(),
        lon_to_json(&compact).unwrap()
    );
    assert_eq!(
        json_to_lon(&lon_to_json(&rendered).unwrap()).unwrap(),
        compact
    );
    assert_eq!(
        pretty.decode(&compact).unwrap(),
        LonCodec::new().decode(&compact).unwrap(),
    );
}

#[test]
fn pretty_lon_reencodes_to_identical_canonical_output() {
    let pretty = LonCodec::new().pretty(true);
    let source = r#"{"a":{"b":{"c":[1,{"d":"e"}]}},"r":[{"x":1,"y":{"z":2}},{"x":3,"y":{"z":4}}]}"#;
    let canonical = json_to_lon(source).unwrap();
    let rendered = pretty.json_to_lon(source).unwrap();
    assert_eq!(
        json_to_lon(&lon_to_json(&rendered).unwrap()).unwrap(),
        canonical
    );
}

#[test]
fn pretty_output_has_no_trailing_whitespace() {
    let pretty = LonCodec::new().pretty(true);
    let rendered = pretty
        .json_to_lon(
            r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}],"o":{"a":[1,{"b":2}]}}"#,
        )
        .unwrap();
    for line in rendered.split('\n') {
        assert!(!line.ends_with(' '), "{line:?}");
        assert!(!line.ends_with('\t'), "{line:?}");
    }
    assert!(!rendered.ends_with('\n'));
}

#[test]
fn pretty_codec_emits_standard_indented_json() {
    let pretty = LonCodec::new().pretty(true);
    assert_eq!(
        pretty.lon_to_json("{users:[id name;1 Ada;2 Bob]}").unwrap(),
        "{\n  \"users\": [\n    {\n      \"id\": 1,\n      \"name\": \"Ada\"\n    },\n    {\n      \"id\": 2,\n      \"name\": \"Bob\"\n    }\n  ]\n}",
    );
}

// ---------------------------------------------------------- strict decoding

#[test]
fn rejects_truncated_or_malformed_table_rows() {
    for source in [
        "[id name;1]",
        "[id name;1 Ada;2]",
        "[id;1 2]",
        "[id name;1 Ada 2 Bob]",
        "[id name;1 Ada;]",
        "[id name;(1 Ada)]",
        "[id name;[1 Ada][2 Bob]]",
    ] {
        expect_format_error(lon_to_json(source), source);
    }
}

#[test]
fn rejects_superseded_table_syntaxes() {
    for source in [
        "table(1)",
        "table(1)[id;(1)]",
        "table[1:id;(1)]",
        "table{1:id;(key:1)}",
        "[2|id;(1)(2)]",
        "{2|id;(a:1)(b:2)}",
        "[|id;]",
    ] {
        expect_format_error(lon_to_json(source), source);
    }
}

#[test]
fn rejects_tables_with_fewer_than_two_rows() {
    for source in ["[a b;]", "[a b;1 2]"] {
        expect_format_error(lon_to_json(source), source);
    }
}

#[test]
fn rejects_ambiguous_single_value_table_schemas() {
    for source in ["[a;1;2]", "[a{b};1;2]", "[a empty{};1;2]"] {
        expect_format_error(lon_to_json(source), source);
    }
}

#[test]
fn retains_schema_fields_and_quoted_cell_boundaries() {
    assert_eq!(
        lon_to_json("[id id;1 2;3 4]").unwrap(),
        r#"[{"id":1,"id":2},{"id":3,"id":4}]"#,
    );
    assert_eq!(
        lon_to_json(r#"["a b" c;1 2;3 4]"#).unwrap(),
        r#"[{"a b":1,"c":2},{"a b":3,"c":4}]"#,
    );
    assert_eq!(
        lon_to_json(r#"[v w;"a;b" 1;"c" 2]"#).unwrap(),
        r#"[{"v":"a;b","w":1},{"v":"c","w":2}]"#,
    );
}

#[test]
fn accepts_dynamic_cells_but_keeps_their_encoding_explicit() {
    let source = "[id payload;1 {a:b};2 {x:[1,{b:c}]}]";
    let json = r#"[{"id":1,"payload":{"a":"b"}},{"id":2,"payload":{"x":[1,{"b":"c"}]}}]"#;
    assert_eq!(lon_to_json(source).unwrap(), json);
    assert_eq!(
        json_to_lon(json).unwrap(),
        "[{id:1 payload:{a:b}},{id:2 payload:{x:[1,{b:c}]}}]",
    );
}

#[test]
fn canonicalizes_table_whitespace_and_preserves_ordinary_prefixes() {
    let spaced = "[ a b ; 1 2 ; 3 4 ; 5 6 ]";
    assert_eq!(
        json_to_lon(&lon_to_json(spaced).unwrap()).unwrap(),
        "[a b;1 2;3 4;5 6]"
    );
    assert_eq!(json_to_lon(r#"{"1":"x"}"#).unwrap(), "{1:x}");
    assert_eq!(lon_to_json("{1:x}").unwrap(), r#"{"1":"x"}"#);
    assert_eq!(lon_to_json("table").unwrap(), "\"table\"");
    assert_eq!(lon_to_json("tablex").unwrap(), "\"tablex\"");
}

#[test]
fn accepts_separators_and_canonicalizes_explicit_type_boundaries() {
    assert_eq!(lon_to_json("[true false]").unwrap(), "[true,false]");
    assert_eq!(lon_to_json("[true,false]").unwrap(), "[true,false]");
    assert_eq!(
        json_to_lon(&lon_to_json("[1 Ada true]").unwrap()).unwrap(),
        "[1,Ada,true]"
    );
    assert_eq!(
        json_to_lon(&lon_to_json("[Ada,Bob]").unwrap()).unwrap(),
        "[Ada Bob]"
    );
    assert_eq!(
        lon_to_json("[Ada 2026-08-02]").unwrap(),
        r#"["Ada","2026-08-02"]"#,
    );
}

#[test]
fn rejects_a_trailing_array_comma() {
    expect_format_error(lon_to_json("[true,]"), "[true,]");
}

#[test]
fn rejects_trailing_content() {
    expect_format_error(lon_to_json("{a:1} {b:2}"), "{a:1} {b:2}");
}

#[test]
fn rejects_ambiguous_or_invisible_bare_tokens() {
    for source in [
        "[01]",
        "[undefined]",
        "[NaN]",
        "[a\u{0}b]",
        "[tr\u{2063}ue]",
        r"[a\b]",
        "{a\"b:1}",
        "{a\u{202E}b:1}",
        r"{a\b:1}",
        "[a\"b;1]",
        "\"a\u{2063}b\"",
        "{\"a\u{202E}b\":1}",
    ] {
        expect_format_error(lon_to_json(source), source);
    }

    assert_eq!(
        lon_to_json(r#"["01" "undefined" "a\u0000b"]"#).unwrap(),
        r#"["01","undefined","a\u0000b"]"#,
    );
    assert_eq!(
        lon_to_json(r#"{"a\u202eb":1}"#).unwrap(),
        r#"{"a\u202eb":1}"#
    );
    assert_eq!(lon_to_json(r#"["a\u2063b"]"#).unwrap(), r#"["a\u2063b"]"#);
}

#[test]
fn rejects_a_byte_order_mark_at_the_document_boundary() {
    expect_format_error(lon_to_json("\u{FEFF}{a:1}"), "BOM before LON");
    expect_format_error(json_to_lon("\u{FEFF}{\"a\":1}"), "BOM before JSON");
}

// ------------------------------------------------------------------- limits

#[test]
fn enforces_expansion_limits() {
    let constrained = LonCodec::new().max_expanded_elements(1);
    expect_format_error(constrained.lon_to_json("[1 2]"), "[1 2]");
}

#[test]
fn handles_deeply_nested_table_schemas_in_linear_work() {
    assert!(lon_to_json(&nested_table(40))
        .unwrap()
        .starts_with(r#"[{"a":"#));
    let constrained = LonCodec::new().max_depth(32);
    expect_format_error(constrained.lon_to_json(&nested_table(2000)), "max depth 32");
}

#[test]
fn enforces_schema_field_limits() {
    let constrained = LonCodec::new().max_expanded_elements(1);
    expect_format_error(constrained.lon_to_json("[a b;]"), "[a b;]");
}

#[test]
fn enforces_element_limits_for_tables() {
    let constrained = LonCodec::new().max_expanded_elements(3);
    expect_format_error(constrained.lon_to_json("[a b;1 2;3 4]"), "[a b;1 2;3 4]");
}

#[test]
fn enforces_element_limits_for_json_encoding() {
    let constrained = LonCodec::new().max_expanded_elements(2);
    assert_eq!(constrained.json_to_lon("[1,2]").unwrap(), "[1,2]");
    expect_format_error(constrained.json_to_lon("[1,2,3]"), "[1,2,3]");
}

#[test]
fn enforces_input_limits() {
    let constrained = LonCodec::new().max_input_code_units(4);
    assert_eq!(constrained.lon_to_json("null").unwrap(), "null");
    expect_format_error(constrained.lon_to_json(" null"), " null");
}

#[test]
fn enforces_output_limits() {
    let constrained = LonCodec::new().max_output_code_units(3);
    assert_eq!(constrained.lon_to_json("\"a\"").unwrap(), "\"a\"");
    expect_format_error(constrained.lon_to_json("\"ab\""), "\"ab\"");

    let expansion_constrained = LonCodec::new().max_output_code_units(20);
    expect_format_error(
        expansion_constrained.lon_to_json("[abcdefghij;1;2;3]"),
        "[abcdefghij;1;2;3]",
    );
}

fn nested_table(depth: usize) -> String {
    let openings = "a{".repeat(depth - 1);
    let closings = "}".repeat(depth - 1);
    format!("[{openings}x{closings} y;0 1;2 3]")
}
