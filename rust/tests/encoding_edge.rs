//! Ports of `dart/test/encoding_edges_test.dart`.

use lon::{json_to_lon, lon_to_json, LonCodec};
use serde_json::json;

fn expect_format_error<T: std::fmt::Debug>(result: Result<T, lon::LonError>, context: &str) {
    let Err(error) = result else {
        panic!("expected a format error for {context:?}, got {result:?}")
    };
    assert_eq!(error.kind(), lon::ErrorKind::Format, "context: {context:?}");
}

// ------------------------------------------------ table selection thresholds

#[test]
fn exactly_two_rows_and_two_fields_still_select_a_table() {
    // The byte-cost gate compares table bytes against the ordinary form; for
    // scalar-valued 2x2 records the table form always wins.
    assert_eq!(
        json_to_lon(r#"[{"a":1,"b":2},{"a":3,"b":4}]"#).unwrap(),
        "[a b;1 2;3 4]"
    );
    // Long keys and values keep the 2x2 shape a table.
    let wide = r#"[{"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa":1,"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb":2},{"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa":3,"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb":4}]"#;
    assert_eq!(
        json_to_lon(wide).unwrap(),
        "[aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb;1 2;3 4]"
    );
    // Multi-byte UTF-8 keys compare on UTF-8 cost, still a table.
    assert_eq!(
        json_to_lon(r#"[{"简":1,"体":2},{"简":3,"体":4}]"#).unwrap(),
        "[简 体;1 2;3 4]"
    );
    // Long string values do not change the structural decision.
    let heavy = serde_json::to_string(&json!([
        {"a": "x".repeat(60), "b": "y".repeat(60)},
        {"a": "z".repeat(60), "b": "w".repeat(60)},
    ]))
    .unwrap();
    assert!(json_to_lon(&heavy).unwrap().starts_with("[a b;"));
}

#[test]
fn three_rows_or_three_fields_always_select_a_table() {
    assert_eq!(
        json_to_lon(r#"[{"a":1,"b":2},{"a":3,"b":4},{"a":5,"b":6}]"#).unwrap(),
        "[a b;1 2;3 4;5 6]"
    );
    assert_eq!(
        json_to_lon(r#"[{"a":1,"b":2,"c":3},{"a":4,"b":5,"c":6}]"#).unwrap(),
        "[a b c;1 2 3;4 5 6]"
    );
    // Nested schema fields count toward the 3-field threshold:
    // id + m{x y} is fieldCount 3.
    assert_eq!(
        json_to_lon(r#"[{"id":1,"m":{"x":2,"y":3}},{"id":4,"m":{"x":5,"y":6}}]"#).unwrap(),
        "[id m{x y};1 2 3;4 5 6]"
    );
    // Deeper nesting contributes recursively: a{x{p q}} b is 4 fields.
    assert_eq!(
        json_to_lon(r#"[{"a":{"x":{"p":1,"q":2}},"b":3},{"a":{"x":{"p":4,"q":5}},"b":6}]"#)
            .unwrap(),
        "[a{x{p q}} b;1 2 3;4 5 6]"
    );
}

#[test]
fn single_field_or_single_leaf_schemas_never_select_a_table() {
    // One member per record: valueCount < 2 keeps objects explicit.
    assert_eq!(
        json_to_lon(r#"[{"a":1},{"a":2}]"#).unwrap(),
        "[{a:1},{a:2}]"
    );
    assert_eq!(
        json_to_lon(r#"[{"a":1},{"a":2},{"a":3}]"#).unwrap(),
        "[{a:1},{a:2},{a:3}]"
    );
    // A single nested-object member still totals one leaf value.
    assert_eq!(
        json_to_lon(r#"[{"a":{"b":1}},{"a":{"b":2}}]"#).unwrap(),
        "[{a:{b:1}},{a:{b:2}}]"
    );
    assert_eq!(
        json_to_lon(r#"[{"a":{"b":{"c":1}}},{"a":{"b":{"c":2}}}]"#).unwrap(),
        "[{a:{b:{c:1}}},{a:{b:{c:2}}}]"
    );
    // One leaf field plus an empty nested object also stays explicit.
    assert_eq!(
        json_to_lon(r#"[{"a":{},"c":2},{"a":{},"c":4}]"#).unwrap(),
        "[{a:{} c:2},{a:{} c:4}]"
    );
}

#[test]
fn non_uniform_member_sets_never_select_a_table() {
    // Mixed object/non-object members.
    assert_eq!(
        json_to_lon(r#"[{"a":1,"b":2},3,{"a":4,"b":5}]"#).unwrap(),
        "[{a:1 b:2},3,{a:4 b:5}]"
    );
    assert_eq!(
        json_to_lon(r#"[{"a":1,"b":2},[1],{"a":3,"b":4}]"#).unwrap(),
        "[{a:1 b:2},[1],{a:3 b:4}]"
    );
    // Varying nested object shapes.
    assert_eq!(
        json_to_lon(r#"[{"a":1,"m":{"x":1}},{"a":2,"m":{"x":1,"y":2}}]"#).unwrap(),
        "[{a:1 m:{x:1}},{a:2 m:{x:1 y:2}}]"
    );
    assert_eq!(
        json_to_lon(r#"[{"a":1,"m":{"x":1,"y":2}},{"a":2,"m":{"x":1,"z":2}}]"#).unwrap(),
        "[{a:1 m:{x:1 y:2}},{a:2 m:{x:1 z:2}}]"
    );
    // Same nested keys, different order.
    assert_eq!(
        json_to_lon(r#"[{"a":1,"m":{"x":1,"y":2}},{"a":2,"m":{"y":1,"x":2}}]"#).unwrap(),
        "[{a:1 m:{x:1 y:2}},{a:2 m:{y:1 x:2}}]"
    );
    // Nested member present in one row only.
    assert_eq!(
        json_to_lon(r#"[{"a":1,"m":{}},{"a":2,"m":{"x":1}}]"#).unwrap(),
        "[{a:1 m:{}},{a:2 m:{x:1}}]"
    );
    // Nested object vs non-object in the same member position.
    assert_eq!(
        json_to_lon(r#"[{"a":{"x":1},"b":2},{"a":7,"b":8}]"#).unwrap(),
        "[{a:{x:1} b:2},{a:7 b:8}]"
    );
    // Single-record array is never a table.
    assert_eq!(json_to_lon(r#"[{"a":1,"b":2}]"#).unwrap(), "[{a:1 b:2}]");
    // A record array nested inside another container stays a table.
    assert_eq!(
        json_to_lon(r#"{"r":[{"a":1,"b":2},{"a":3,"b":4}]}"#).unwrap(),
        "{r:[a b;1 2;3 4]}"
    );
}

// -------------------------------------------------------- text API fidelity

#[test]
fn preserves_repeated_members_three_or_more_times_in_order() {
    let source = r#"{"a":1,"b":2,"a":3,"a":4,"c":5,"a":6}"#;
    let lon = json_to_lon(source).unwrap();
    assert_eq!(lon, "{a:1 b:2 a:3 a:4 c:5 a:6}");
    assert_eq!(lon_to_json(&lon).unwrap(), source);
    assert_eq!(json_to_lon(&lon_to_json(&lon).unwrap()).unwrap(), lon);
    // The value API collapses duplicates with the last value winning.
    assert_eq!(
        lon_to_json("{a:1 a:2 a:3}").unwrap(),
        r#"{"a":1,"a":2,"a":3}"#
    );
    assert_eq!(
        LonCodec::new().decode("{a:1 b:2 a:3}").unwrap(),
        json!({"a": 3, "b": 2})
    );
}

#[test]
fn preserves_extreme_number_lexemes_verbatim() {
    let source = r#"[{"a":-0,"b":0.0e0,"c":1e+09,"d":-0.0,"e":1E-3},{"a":1e+09,"b":-0,"c":0.0e0,"d":-0.0,"e":1E-3}]"#;
    let lon = json_to_lon(source).unwrap();
    assert_eq!(
        lon,
        "[a b c d e;-0 0.0e0 1e+09 -0.0 1E-3;1e+09 -0 0.0e0 -0.0 1E-3]"
    );
    assert_eq!(lon_to_json(&lon).unwrap(), source);
}

#[test]
fn preserves_a_huge_integer_lexeme_through_the_text_pipeline() {
    let source = r#"{"n":123456789012345678901234567890}"#;
    let lon = json_to_lon(source).unwrap();
    assert_eq!(lon, "{n:123456789012345678901234567890}");
    assert_eq!(lon_to_json(&lon).unwrap(), source);
    assert_eq!(json_to_lon(&lon_to_json(&lon).unwrap()).unwrap(), lon);
    // The value API degrades gracefully to f64.
    assert_eq!(
        LonCodec::new().decode(&lon).unwrap(),
        json!({"n": 1.2345678901234568e29})
    );
}

#[test]
fn preserves_empty_member_names() {
    let source = r#"{"":1,"a":{"":2}}"#;
    let lon = json_to_lon(source).unwrap();
    assert_eq!(lon, r#"{"":1 a:{"":2}}"#);
    assert_eq!(lon_to_json(&lon).unwrap(), source);
    assert_eq!(json_to_lon(&lon_to_json(&lon).unwrap()).unwrap(), lon);
    assert_eq!(
        LonCodec::new().decode(&lon).unwrap(),
        json!({"": 1, "a": {"": 2}})
    );
}

#[test]
fn preserves_escaped_and_surrogate_pair_member_names() {
    let source = r#"{"\ud800":1,"a\u0000b":2,"😀":3}"#;
    let lon = json_to_lon(source).unwrap();
    assert_eq!(lon, "{\"\\ud800\":1 \"a\\u0000b\":2 😀:3}");
    assert_eq!(lon_to_json(&lon).unwrap(), source);
    assert_eq!(json_to_lon(&lon_to_json(&lon).unwrap()).unwrap(), lon);
}

#[test]
fn interleaves_escaped_strings_across_nested_object_in_array_members() {
    let source = r#"[{"a":"x\ty","b":{"c":"u\nv"},"d":[1,{"e":"q\"r"}],"f":2},{"a":"x\ty","b":{"c":"u\nv"},"d":[2,{"e":"q\"r"}],"f":3}]"#;
    let lon = json_to_lon(source).unwrap();
    assert_eq!(
        lon,
        r#"[a b{c} d f;"x\ty" "u\nv" [1,{e:"q\"r"}] 2;"x\ty" "u\nv" [2,{e:"q\"r"}] 3]"#
    );
    assert_eq!(lon_to_json(&lon).unwrap(), source);
    assert_eq!(json_to_lon(&lon_to_json(&lon).unwrap()).unwrap(), lon);
}

// -------------------------------------------------- ambiguity quoting rules

#[test]
fn quotes_every_json_adjacent_literal_as_a_string_value() {
    const TOKENS: [&str; 32] = [
        "NaN",
        "nan",
        "Infinity",
        "inf",
        "-Infinity",
        "+Infinity",
        "-inf",
        ".NaN",
        "+.inf",
        "undefined",
        "null",
        "NULL",
        "true",
        "TRUE",
        "false",
        "False",
        "None",
        "none",
        "nil",
        "yes",
        "YES",
        "no",
        "NO",
        "on",
        "ON",
        "off",
        "OFF",
        "y",
        "Y",
        "n",
        "N",
        "~",
    ];
    for token in TOKENS {
        let json = serde_json::to_string(token).unwrap();
        assert_eq!(json_to_lon(&json).unwrap(), json, "token {token}");
    }
    // Single-letter JSON-adjacent members not in the reserved set stay bare.
    assert_eq!(json_to_lon("\"T\"").unwrap(), "T");
    assert_eq!(json_to_lon("\"F\"").unwrap(), "F");
    assert_eq!(json_to_lon("\"x\"").unwrap(), "x");
}

#[test]
fn applies_the_apostrophe_rule_to_string_values() {
    // An apostrophe in the middle of a token stays bare.
    assert_eq!(json_to_lon("\"don't\"").unwrap(), "don't");
    assert_eq!(json_to_lon("\"O'Reilly\"").unwrap(), "O'Reilly");
    // Leading or trailing apostrophes force quoting.
    assert_eq!(json_to_lon("\"'abc\"").unwrap(), "\"'abc\"");
    assert_eq!(json_to_lon("\"abc'\"").unwrap(), "\"abc'\"");
    assert_eq!(json_to_lon("\"'abc'\"").unwrap(), "\"'abc'\"");
    assert_eq!(json_to_lon("\"'\"").unwrap(), "\"'\"");
    // Round-trip through the bare-token side.
    assert_eq!(lon_to_json("don't").unwrap(), "\"don't\"");
    expect_format_error(lon_to_json("'abc"), "leading apostrophe");
    expect_format_error(lon_to_json("abc'"), "trailing apostrophe");
}

#[test]
fn quotes_strings_containing_structural_punctuation() {
    for punctuation in ["(", ")", "[", "]", "{", "}", ",", ":", ";", "|", "\"", "\\"] {
        let value = format!("a{punctuation}b");
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json_to_lon(&json).unwrap(), json, "token {value}");
    }
    // An entire token of punctuation also quotes.
    assert_eq!(json_to_lon("\",\"").unwrap(), "\",\"");
    assert_eq!(json_to_lon("\"()\"").unwrap(), "\"()\"");
    assert_eq!(
        json_to_lon("\"a:b;c,d|e[f]g{h}(i)\"").unwrap(),
        "\"a:b;c,d|e[f]g{h}(i)\""
    );
}

#[test]
fn quotes_numeric_looking_strings_that_json_would_misread() {
    const QUOTED: [&str; 14] = [
        "1e", "1e+", "1.", ".5", "._", "0x10", "0X10", "0b101", "0o77", "+1", "-01", "01", "1_000",
        "1.e3",
    ];
    for token in QUOTED {
        let json = serde_json::to_string(token).unwrap();
        assert_eq!(json_to_lon(&json).unwrap(), json, "token {token}");
    }
    // Bare forms of those tokens must be rejected at parse time.
    for token in ["1e", ".5", "0x10", "+1", "01", "1_000"] {
        let source = format!("[{token}]");
        expect_format_error(lon_to_json(&source), &format!("bare {token}"));
    }
    // JSON number spellings still encode as numbers.
    for lexeme in ["0", "-0", "1", "-12.5e+2", "1e+02", "10"] {
        assert_eq!(json_to_lon(lexeme).unwrap(), lexeme, "lexeme {lexeme}");
    }
    // Non-numeric punctuation-free tokens stay bare.
    for token in [
        "1-2",
        "e",
        "0x",
        "Infinityx",
        "1x",
        "x1",
        "a1e",
        "1_",
        "1_.5",
        "1__2",
    ] {
        let json = serde_json::to_string(token).unwrap();
        assert_eq!(json_to_lon(&json).unwrap(), token, "token {token}");
    }
}
