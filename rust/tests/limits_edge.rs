//! Ports of `dart/test/limits_edge_test.dart`.

use lon::{ErrorKind, LonCodec, LonError};
use serde_json::json;

fn expect_format_error<T: std::fmt::Debug>(result: Result<T, LonError>, context: &str) {
    let Err(error) = result else {
        panic!("expected a format error for {context:?}, got {result:?}")
    };
    assert_eq!(error.kind(), ErrorKind::Format, "context: {context:?}");
}

fn expect_invalid_value<T: std::fmt::Debug>(result: Result<T, LonError>, context: &str) {
    let Err(error) = result else {
        panic!("expected an invalid-value error for {context:?}, got {result:?}")
    };
    assert_eq!(
        error.kind(),
        ErrorKind::InvalidValue,
        "context: {context:?}"
    );
}

// -------------------------------------------------------------- depth limits

// Table rows expand into real objects: the schema's deepest field path adds to
// the depth of the table itself.
#[test]
fn counts_expanded_table_schema_depth_against_max_depth() {
    // Schema a{b{c{d{x y}}} z} w has a maximum field path of 4
    // (a -> b -> c -> d -> x/y); the table sits at depth 0, so the deepest leaf
    // value lands at depth 5.
    let source = "[a{b{c{d{x y}}} z} w;0 1 2 3;4 5 6 7]";
    let expected = r#"[{"a":{"b":{"c":{"d":{"x":0,"y":1}}},"z":2},"w":3},{"a":{"b":{"c":{"d":{"x":4,"y":5}}},"z":6},"w":7}]"#;

    for allowed in 0..=5 {
        expect_format_error(
            LonCodec::new().max_depth(allowed).lon_to_json(source),
            &format!("maxDepth {allowed}"),
        );
    }
    assert_eq!(
        LonCodec::new().max_depth(6).lon_to_json(source).unwrap(),
        expected
    );
    assert_eq!(
        LonCodec::new().max_depth(7).lon_to_json(source).unwrap(),
        expected
    );
}

#[test]
fn applies_max_depth_identically_on_all_three_entry_points() {
    let three = LonCodec::new().max_depth(3);
    assert_eq!(three.decode("[[[1]]]").unwrap(), json!([[[1]]]),);
    expect_format_error(three.decode("[[[[1]]]]"), "decode [[[[1]]]]");
    assert_eq!(three.lon_to_json("[[[1]]]").unwrap(), "[[[1]]]");
    expect_format_error(three.json_to_lon("[[[[1]]]]"), "json_to_lon [[[[1]]]]");
    assert_eq!(three.encode(&json!([[[2]]])).unwrap(), "[[[2]]]");
    expect_invalid_value(three.encode(&json!([[[[2]]]])), "encode [[[[2]]]]");
}

#[test]
fn zero_depth_accepts_scalars_and_empty_containers_only() {
    let flat = LonCodec::new().max_depth(0);
    assert_eq!(flat.decode("null").unwrap(), json!(null));
    assert_eq!(flat.decode("[]").unwrap(), json!([]));
    assert_eq!(flat.decode("{}").unwrap(), json!({}));
    expect_format_error(flat.lon_to_json("[a b;1 2;3 4]"), "table at depth 0");
    expect_format_error(flat.decode("[[1]]"), "[[1]]");
    expect_format_error(flat.decode("{a:1}"), "{a:1}");
    expect_invalid_value(flat.encode(&json!([1])), "encode [1]");
    expect_invalid_value(flat.encode(&json!({"a": 1})), "encode {a:1}");
    expect_format_error(flat.json_to_lon("[1]"), "json_to_lon [1]");
    // Table schema nesting counts against depth even at 0.
    expect_format_error(
        flat.lon_to_json("[a{x} b;1 2;3 4]"),
        "nested schema at depth 0",
    );
}

// --------------------------------------------------- expanded element limits

#[test]
fn bounds_ordinary_lon_array_elements_exactly_at_the_limit() {
    let two = LonCodec::new().max_expanded_elements(2);
    assert_eq!(two.lon_to_json("[1 2]").unwrap(), "[1,2]");
    expect_format_error(two.lon_to_json("[1 2 3]"), "[1 2 3]");
    let three = LonCodec::new().max_expanded_elements(3);
    assert_eq!(three.lon_to_json("[1 2 3]").unwrap(), "[1,2,3]");
}

#[test]
fn bounds_ordinary_lon_object_members_exactly_at_the_limit() {
    let two = LonCodec::new().max_expanded_elements(2);
    assert_eq!(two.lon_to_json("{a:1 b:2}").unwrap(), r#"{"a":1,"b":2}"#);
    expect_format_error(two.lon_to_json("{a:1 b:2 c:3}"), "{a:1 b:2 c:3}");
}

#[test]
fn bounds_json_parsed_elements_on_encode_exactly_at_the_limit() {
    let two = LonCodec::new().max_expanded_elements(2);
    assert_eq!(two.json_to_lon("[1,2]").unwrap(), "[1,2]");
    expect_format_error(two.json_to_lon("[1,2,3]"), "[1,2,3]");
    assert_eq!(two.json_to_lon(r#"{"a":1,"b":2}"#).unwrap(), "{a:1 b:2}");
    expect_format_error(two.json_to_lon(r#"{"a":1,"b":2,"c":3}"#), "three members");
    // Nested elements count too.
    let five = LonCodec::new().max_expanded_elements(5);
    assert_eq!(five.json_to_lon("[1,[2,[3]]]").unwrap(), "[1,[2,[3]]]");
    let four = LonCodec::new().max_expanded_elements(4);
    expect_format_error(four.json_to_lon("[1,[2,[3]]]"), "nested [1,[2,[3]]]");
}

#[test]
fn bounds_value_graph_elements_on_encode_exactly_at_the_limit() {
    let three = LonCodec::new().max_expanded_elements(3);
    assert_eq!(three.encode(&json!([1, 2, 3])).unwrap(), "[1,2,3]");
    let two = LonCodec::new().max_expanded_elements(2);
    assert_eq!(two.encode(&json!({"a": 1, "b": 2})).unwrap(), "{a:1 b:2}");
    expect_invalid_value(two.encode(&json!([1, 2, 3])), "encode [1,2,3]");
    expect_invalid_value(
        two.encode(&json!({"a": 1, "b": 2, "c": 3})),
        "encode three members",
    );
}

#[test]
fn counts_each_table_row_as_field_count_plus_one_expanded_elements() {
    // [a b;...] -> 3 slots per row: two rows fit in 6, the third needs 9.
    let six = LonCodec::new().max_expanded_elements(6);
    assert_eq!(
        six.lon_to_json("[a b;1 2;3 4]").unwrap(),
        r#"[{"a":1,"b":2},{"a":3,"b":4}]"#,
    );
    expect_format_error(six.lon_to_json("[a b;1 2;3 4;5 6]"), "third row");
    let nine = LonCodec::new().max_expanded_elements(9);
    assert_eq!(
        nine.lon_to_json("[a b;1 2;3 4;5 6]").unwrap(),
        r#"[{"a":1,"b":2},{"a":3,"b":4},{"a":5,"b":6}]"#,
    );
    let eight = LonCodec::new().max_expanded_elements(8);
    expect_format_error(eight.lon_to_json("[a b;1 2;3 4;5 6]"), "budget 8");
}

#[test]
fn counts_schema_fields_against_their_own_element_budget() {
    // Schema fields are tracked by a separate counter bounded by the same
    // limit: a 3-field schema already fails at budget 2 even before rows.
    let two = LonCodec::new().max_expanded_elements(2);
    expect_format_error(two.lon_to_json("[a b c;1 2 3;4 5 6]"), "budget 2");
    let three = LonCodec::new().max_expanded_elements(3);
    expect_format_error(three.lon_to_json("[a b c;1 2 3;4 5 6]"), "budget 3");
    // The schema fits the field counter at 3, but each row then needs
    // fieldCount + 1 = 4 expanded elements, so 2 rows need 8.
    let eight = LonCodec::new().max_expanded_elements(8);
    assert_eq!(
        eight.lon_to_json("[a b c;1 2 3;4 5 6]").unwrap(),
        r#"[{"a":1,"b":2,"c":3},{"a":4,"b":5,"c":6}]"#,
    );
    let seven = LonCodec::new().max_expanded_elements(7);
    expect_format_error(seven.lon_to_json("[a b c;1 2 3;4 5 6]"), "budget 7");
    // Nested schema fields count recursively: a{x y} b is 4 schema fields and
    // 4 + 1 = 5 expanded elements per row.
    let four_fields = LonCodec::new().max_expanded_elements(3);
    expect_format_error(
        four_fields.lon_to_json("[a{x y} b;1 2 3;4 5 6]"),
        "nested schema budget 3",
    );
    let nested_budget = LonCodec::new().max_expanded_elements(2 * 5);
    assert_eq!(
        nested_budget.lon_to_json("[a{x y} b;1 2 3;4 5 6]").unwrap(),
        r#"[{"a":{"x":1,"y":2},"b":3},{"a":{"x":4,"y":5},"b":6}]"#,
    );
    let nested_short = LonCodec::new().max_expanded_elements(2 * 5 - 1);
    expect_format_error(
        nested_short.lon_to_json("[a{x y} b;1 2 3;4 5 6]"),
        "nested schema budget 9",
    );
}

#[test]
fn zero_element_budget_accepts_only_empty_containers_and_scalars() {
    let zero = LonCodec::new().max_expanded_elements(0);
    assert_eq!(zero.lon_to_json("null").unwrap(), "null");
    assert_eq!(zero.lon_to_json("[]").unwrap(), "[]");
    assert_eq!(zero.lon_to_json("{}").unwrap(), "{}");
    assert_eq!(zero.encode(&json!(null)).unwrap(), "null");
    assert_eq!(zero.encode(&json!([])).unwrap(), "[]");
    assert_eq!(zero.encode(&json!({})).unwrap(), "{}");
    expect_format_error(zero.lon_to_json("[1]"), "[1]");
    expect_format_error(zero.lon_to_json("{a:1}"), "{a:1}");
    expect_format_error(zero.lon_to_json("[a b;1 2;3 4]"), "table");
    expect_invalid_value(zero.encode(&json!([1])), "encode [1]");
    expect_invalid_value(zero.encode(&json!({"a": 1})), "encode {a:1}");
    expect_format_error(zero.json_to_lon("[1]"), "json_to_lon [1]");
    expect_format_error(zero.json_to_lon(r#"{"a":1}"#), r#"json_to_lon {"a":1}"#);
}

// ------------------------------------------- input and output code-unit caps

#[test]
fn accepts_input_exactly_at_the_limit_and_rejects_limit_plus_one() {
    let four = LonCodec::new().max_input_code_units(4);
    assert_eq!(four.lon_to_json("null").unwrap(), "null");
    assert_eq!(four.decode("null").unwrap(), json!(null));
    assert_eq!(four.json_to_lon("null").unwrap(), "null");
    expect_format_error(four.lon_to_json(" null"), "lon ' null'");
    expect_format_error(four.decode(" null"), "decode ' null'");
    expect_format_error(four.json_to_lon(" null"), "json ' null'");
}

#[test]
fn counts_input_in_utf_16_code_units_not_characters() {
    // The emoji is 2 code units: 'a' + emoji = 3 units inside quotes.
    let five = LonCodec::new().max_input_code_units(5);
    assert_eq!(five.decode("\"a\u{1F600}\"").unwrap(), json!("a\u{1F600}"));
    expect_format_error(five.decode("\"ab\u{1F600}\""), "ab + emoji");
}

#[test]
fn accepts_output_exactly_at_the_limit_and_rejects_limit_plus_one() {
    let three = LonCodec::new().max_output_code_units(3);
    assert_eq!(three.encode(&json!("Ada")).unwrap(), "Ada");
    // The serialized output limit is a Format violation, like the Dart codec's
    // FormatException on encode.
    expect_format_error(three.encode(&json!("Alan")), "encode Alan");
    assert_eq!(three.lon_to_json("\"a\"").unwrap(), "\"a\"");
    expect_format_error(three.lon_to_json("\"ab\""), "\"ab\"");
    // Output escaping also counts against the budget: '"\\t"' is 4 units.
    let four = LonCodec::new().max_output_code_units(4);
    assert_eq!(four.lon_to_json(r#""\t""#).unwrap(), r#""\t""#);
    let tight = LonCodec::new().max_output_code_units(3);
    expect_format_error(tight.lon_to_json(r#""\t""#), r#""\t""#);
}

#[test]
fn zero_limits_permit_only_empty_output_and_empty_input() {
    let zero_in = LonCodec::new().max_input_code_units(0);
    expect_format_error(zero_in.lon_to_json("null"), "lon null");
    expect_format_error(zero_in.decode("null"), "decode null");
    expect_format_error(zero_in.json_to_lon("null"), "json null");
    expect_format_error(zero_in.lon_to_json(""), "empty input");

    let zero_out = LonCodec::new().max_output_code_units(0);
    // Empty-string and null encodings are non-empty output, so all fail with
    // the output-limit Format violation.
    expect_format_error(zero_out.encode(&json!("")), "encode ''");
    expect_format_error(zero_out.encode(&json!(null)), "encode null");
    expect_format_error(zero_out.lon_to_json(r#""""#), "lon empty string");
    expect_format_error(zero_out.lon_to_json("null"), "lon null");
    expect_format_error(zero_out.json_to_lon(r#""""#), "json empty string");
}

#[test]
fn serde_entry_points_share_the_value_limits() {
    // to_string/from_str route through the same codec limits.
    let limited = LonCodec::new().max_expanded_elements(2);
    assert_eq!(limited.to_string(&vec![1u8, 2]).unwrap(), "[1,2]");
    assert!(limited.to_string(&vec![1u8, 2, 3]).is_err());
    let parsed: Vec<u8> = limited.from_str("[1 2]").unwrap();
    assert_eq!(parsed, vec![1, 2]);
    assert!(limited.from_str::<Vec<u8>>("[1 2 3]").is_err());

    // An object member counts as exactly one element on every entry point,
    // including the serde path: one member fits budget 1, two do not.
    let one = LonCodec::new().max_expanded_elements(1);
    assert_eq!(one.encode(&json!({"a": 1})).unwrap(), "{a:1}");
    assert_eq!(one.to_string(&json!({"a": 1})).unwrap(), "{a:1}");
    expect_invalid_value(one.to_string(&json!({"a": 1, "b": 2})), "serde two members");
}
