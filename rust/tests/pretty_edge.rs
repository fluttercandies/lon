//! Ports of `dart/test/pretty_edge_test.dart`.

use lon::{json_to_lon, lon_to_json, LonCodec};
use serde_json::json;

fn pretty() -> LonCodec {
    LonCodec::new().pretty(true)
}

#[test]
fn empty_containers_stay_inline_at_deep_nesting() {
    assert_eq!(
        pretty()
            .json_to_lon(r#"{"o":{"e":{},"a":[]},"m":{"x":1}}"#)
            .unwrap(),
        "{\n  o: {\n    e: {}\n    a: []\n  }\n  m: {\n    x: 1\n  }\n}",
    );
    assert_eq!(
        pretty().json_to_lon(r#"{"e":{},"l":[]}"#).unwrap(),
        "{\n  e: {}\n  l: []\n}",
    );
    assert_eq!(
        pretty().json_to_lon("[[{}]]").unwrap(),
        "[\n  [\n    {}\n  ]\n]",
    );
    assert_eq!(
        pretty().json_to_lon(r#"{"a":[[],[{}]]}"#).unwrap(),
        "{\n  a: [\n    [],\n    [\n      {}\n    ]\n  ]\n}",
    );
}

#[test]
fn pure_scalar_arrays_stay_on_one_line_in_multiline_contexts() {
    assert_eq!(
        pretty()
            .json_to_lon(r#"{"row":[1,2,3],"next":[{"a":1}]}"#)
            .unwrap(),
        "{\n  row: [1,2,3]\n  next: [\n    {\n      a: 1\n    }\n  ]\n}",
    );
    assert_eq!(
        pretty()
            .json_to_lon(r#"{"tags":["x","y"],"nums":[0.5,-1,2e3]}"#)
            .unwrap(),
        "{\n  tags: [x,\"y\"]\n  nums: [0.5,-1,2e3]\n}",
    );
}

#[test]
fn nested_table_inside_a_multiline_array_keeps_cells_on_one_row() {
    assert_eq!(
        pretty()
            .json_to_lon(
                r#"{"outer":{"rows":[[{"s":"x","q":1},{"s":"y","q":2}],{"k":"v"}],"z":1}}"#,
            )
            .unwrap(),
        "{\n  outer: {\n    rows: [\n      [\n        s q;\n        x 1;\n        \"y\" 2\n      ],\n      {\n        k: v\n      }\n    ]\n    z: 1\n  }\n}",
    );
}

#[test]
fn table_cells_holding_nested_tables_render_inline() {
    assert_eq!(
        pretty()
            .json_to_lon(
                r#"{"o":[{"id":1,"items":[{"s":"x","q":2},{"s":"y","q":1},{"s":"z","q":3}]},{"id":2,"items":[{"s":"w","q":4},{"s":"v","q":5},{"s":"u","q":6}]}]}"#,
            )
            .unwrap(),
        "{\n  o: [\n    id items;\n    1 [s q;x 2;\"y\" 1;z 3];\n    2 [s q;w 4;v 5;u 6]\n  ]\n}",
    );
}

#[test]
fn object_inside_a_table_cell_renders_inline() {
    assert_eq!(
        pretty()
            .json_to_lon(r#"[{"id":1,"payload":{"a":{"b":2}}},{"id":2,"payload":{"a":{"b":3}}}]"#,)
            .unwrap(),
        "[\n  id payload{a{b}};\n  1 2;\n  2 3\n]",
    );
}

#[test]
fn pretty_decode_emits_two_space_indented_json_at_every_depth() {
    let codec = pretty();
    assert_eq!(codec.lon_to_json("[]").unwrap(), "[]");
    assert_eq!(codec.lon_to_json("{}").unwrap(), "{}");
    assert_eq!(codec.lon_to_json("null").unwrap(), "null");
    assert_eq!(codec.lon_to_json("\"a\"").unwrap(), "\"a\"");
    assert_eq!(
        codec.lon_to_json("[a b;1 2;3 4]").unwrap(),
        "[\n  {\n    \"a\": 1,\n    \"b\": 2\n  },\n  {\n    \"a\": 3,\n    \"b\": 4\n  }\n]",
    );
    assert_eq!(
        codec.lon_to_json("{a:[1 2] b:{}}").unwrap(),
        "{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": {}\n}",
    );
    assert_eq!(
        codec.lon_to_json("[[a b;1 2;3 4]]").unwrap(),
        "[\n  [\n    {\n      \"a\": 1,\n      \"b\": 2\n    },\n    {\n      \"a\": 3,\n      \"b\": 4\n    }\n  ]\n]",
    );
}

#[test]
fn pretty_output_decodes_to_the_same_values_as_compact_output() {
    let codec = pretty();
    let source = r#"{"outer":{"rows":[[{"s":"x","q":1},{"s":"y","q":2}],{"k":"v"}],"z":1},"empty":{"o":{},"a":[]},"scalars":[1,2.5,"x"],"t":[{"a":1,"b":{"c":2}},{"a":3,"b":{"c":4}}]}"#;
    let compact_lon = json_to_lon(source).unwrap();
    let pretty_lon = codec.json_to_lon(source).unwrap();

    assert_eq!(
        lon_to_json(&pretty_lon).unwrap(),
        lon_to_json(&compact_lon).unwrap()
    );
    // Canonical LON re-encoded from pretty output is byte-identical.
    assert_eq!(
        json_to_lon(&lon_to_json(&pretty_lon).unwrap()).unwrap(),
        compact_lon
    );
    // The pretty codec re-encodes to the identical pretty text (idempotent).
    assert_eq!(
        codec
            .json_to_lon(&lon_to_json(&pretty_lon).unwrap())
            .unwrap(),
        pretty_lon
    );
}

#[test]
fn lon_whitespace_members_parse_identically_to_a_plain_space() {
    const WHITESPACE: [u32; 25] = [
        0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x20, 0x85, 0xA0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003,
        0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200A, 0x2028, 0x2029, 0x202F, 0x205F,
        0x3000,
    ];
    const EXPECTED: &str = r#"{"a":1,"b":[true,{"c":null}],"d":{"e":2,"f":3}}"#;
    for code in WHITESPACE {
        let ws = char::from_u32(code).unwrap();
        let source = format!("{{a:{ws}1{ws}b:[true,{ws}{{c:null}}]{ws}d:{{e:{ws}2{ws}f:3}}}}");
        assert_eq!(
            lon_to_json(&source).unwrap(),
            EXPECTED,
            "whitespace U+{code:04X}"
        );
    }

    // A multiline pretty document re-parses to the same value with any mix of
    // TAB, CRLF, and Unicode whitespace substituted for spaces.
    let mixed_pretty =
        "{\n\touter:\u{a0}{\r\n  rows:\u{2028}[\n    a\u{3000}b;\n    1\u{202f}2;\n    3\u{2007}4\n  ]\n}\n}";
    assert_eq!(
        LonCodec::new().decode(mixed_pretty).unwrap(),
        json!({"outer": {"rows": [{"a": 1, "b": 2}, {"a": 3, "b": 4}]}}),
    );
}
