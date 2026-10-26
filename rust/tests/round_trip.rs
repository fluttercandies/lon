//! Ports of the deterministic structural tests in
//! `dart/test/round_trip_test.dart`.

use lon::{json_to_lon, lon_to_json, LonCodec};
use serde_json::{json, Value};

fn expect_all_round_trip(value: &Value, codec: &LonCodec, reason: &str) -> String {
    let json = serde_json::to_string(value).unwrap();

    let encoded = codec.encode(value).unwrap();
    assert_eq!(
        codec.decode(&encoded).unwrap(),
        *value,
        "{reason}: value round-trip",
    );

    let text_encoded = codec.json_to_lon(&json).unwrap();
    assert_eq!(
        text_encoded, encoded,
        "{reason}: value and JSON encoders agree"
    );

    let decoded_json = codec.lon_to_json(&text_encoded).unwrap();
    // Re-parse through the codec itself: serde_json's own text parser caps
    // recursion at 128, below the depths under test.
    let reparsed = codec
        .decode(&codec.json_to_lon(&decoded_json).unwrap())
        .unwrap();
    assert_eq!(reparsed, *value, "{reason}: JSON semantic round-trip");
    assert_eq!(
        codec.json_to_lon(&decoded_json).unwrap(),
        text_encoded,
        "{reason}: canonical LON is idempotent",
    );
    text_encoded
}

fn expect_json_text_round_trip(json: &str) -> String {
    let encoded = json_to_lon(json).unwrap();
    let decoded = lon_to_json(&encoded).unwrap();
    assert_eq!(decoded, json);
    assert_eq!(json_to_lon(&decoded).unwrap(), encoded);
    encoded
}

fn nested_array(depth: usize) -> Value {
    let mut value = json!(0);
    for _ in 0..depth {
        value = json!([value]);
    }
    value
}

fn nested_object(depth: usize) -> Value {
    let mut value = json!(0);
    for _ in 0..depth {
        value = json!({"value": value});
    }
    value
}

fn nested_array_json(depth: usize) -> String {
    format!("{}0{}", "[".repeat(depth), "]".repeat(depth))
}

fn nested_object_json(depth: usize) -> String {
    format!("{}0{}", "{\"value\":".repeat(depth), "}".repeat(depth))
}

#[test]
fn ordinary_arrays_and_objects_honor_exact_depth_boundaries() {
    let zero_depth = LonCodec::new().max_depth(0).max_expanded_elements(2048);
    expect_all_round_trip(&json!([]), &zero_depth, "empty array");
    expect_all_round_trip(&json!({}), &zero_depth, "empty object");

    for depth in [1usize, 2, 31, 32, 127, 511, 512] {
        let cases = [
            ("array", nested_array(depth), nested_array_json(depth)),
            ("object", nested_object(depth), nested_object_json(depth)),
        ];
        for (kind, value, json) in cases {
            let codec = LonCodec::new().max_depth(depth).max_expanded_elements(2048);
            let encoded = expect_all_round_trip(&value, &codec, &format!("{kind} depth {depth}"));
            let too_shallow = LonCodec::new()
                .max_depth(depth - 1)
                .max_expanded_elements(2048);
            assert!(
                too_shallow.encode(&value).is_err(),
                "{kind} value at depth {}",
                depth - 1,
            );
            assert!(
                too_shallow.json_to_lon(&json).is_err(),
                "{kind} JSON at depth {}",
                depth - 1,
            );
            assert!(
                too_shallow.lon_to_json(&encoded).is_err(),
                "{kind} LON at depth {}",
                depth - 1,
            );
        }
    }

    let codec = LonCodec::new();
    assert!(codec.encode(&nested_array(513)).is_err());
    assert!(codec.encode(&nested_object(513)).is_err());
    assert!(json_to_lon(&nested_array_json(513)).is_err());
    assert!(json_to_lon(&nested_object_json(513)).is_err());
}

#[test]
fn round_trips_every_ordered_dynamic_field_type_transition() {
    let transitions: Vec<(&str, Value)> = vec![
        ("null", json!(null)),
        ("boolean", json!(true)),
        ("integer", json!(-7)),
        ("double", json!(1.25)),
        ("string", json!("a b;[]{}")),
        ("empty array", json!([])),
        ("array", json!([1, "x"])),
        ("empty object", json!({})),
        ("flat object", json!({"a": 1})),
        ("nested object", json!({"x": [1, {"y": "z"}]})),
    ];

    for (left_name, left) in &transitions {
        for (right_name, right) in &transitions {
            let value = json!([
                {"id": 1, "payload": left},
                {"id": 2, "payload": right},
            ]);
            let reason = format!("{left_name} -> {right_name}");
            let encoded = expect_all_round_trip(&value, &LonCodec::new(), &reason);
            let left_is_object = matches!(left, Value::Object(_));
            let right_is_object = matches!(right, Value::Object(_));
            if left_is_object != right_is_object {
                assert!(encoded.starts_with("[{"), "{reason}");
            }
        }
    }
}

#[test]
fn selects_tables_only_for_recursively_uniform_record_structures() {
    let cases: Vec<(&str, Value, bool)> = vec![
        (
            "same keys and order",
            json!([{"a": 1, "b": 2, "c": 3}, {"a": 4, "b": 5, "c": 6}]),
            true,
        ),
        (
            "same keys in different order",
            json!([{"a": 1, "b": 2, "c": 3}, {"b": 4, "a": 5, "c": 6}]),
            false,
        ),
        (
            "missing key",
            json!([{"a": 1, "b": 2, "c": 3}, {"a": 4, "b": 5}]),
            false,
        ),
        (
            "extra key",
            json!([{"a": 1, "b": 2}, {"a": 3, "b": 4, "c": 5}]),
            false,
        ),
        (
            "recursively uniform nested objects",
            json!([
                {"id": 1, "meta": {"x": 2, "y": 3}, "ok": true},
                {"id": 4, "meta": {"x": 5, "y": 6}, "ok": false},
            ]),
            true,
        ),
        (
            "different nested keys",
            json!([
                {"id": 1, "meta": {"x": 2, "y": 3}},
                {"id": 4, "meta": {"x": 5, "z": 6}},
            ]),
            false,
        ),
        (
            "different nested key order",
            json!([
                {"id": 1, "meta": {"x": 2, "y": 3}},
                {"id": 4, "meta": {"y": 5, "x": 6}},
            ]),
            false,
        ),
        (
            "empty versus non-empty nested object",
            json!([{"id": 1, "meta": {}}, {"id": 2, "meta": {"x": 3}}]),
            false,
        ),
        (
            "varying array cells remain table values",
            json!([
                {"id": 1, "payload": [1], "kind": "short"},
                {"id": 2, "payload": [{"x": 3}], "kind": "nested"},
            ]),
            true,
        ),
        (
            "record and non-record mixture",
            json!([{"a": 1}, 2, {"a": 3}]),
            false,
        ),
    ];

    for (name, value, uses_table) in cases {
        let encoded = expect_all_round_trip(&value, &LonCodec::new(), name);
        assert_eq!(has_top_level_table(&encoded), uses_table, "{name}");
    }
}

#[test]
fn preserves_nested_duplicate_record_keys_and_order_in_text_apis() {
    let uniform = r#"[{"a":1,"a":2,"meta":{"x":3}},{"a":4,"a":5,"meta":{"x":6}}]"#;
    let reordered = r#"[{"a":1,"a":2,"meta":{"x":3}},{"a":4,"meta":{"x":6},"a":5}]"#;

    let uniform_lon = expect_json_text_round_trip(uniform);
    let reordered_lon = expect_json_text_round_trip(reordered);

    assert!(has_top_level_table(&uniform_lon));
    assert!(!has_top_level_table(&reordered_lon));
}

#[test]
fn rejects_every_truncation_of_complex_root_containers() {
    let sources = [
        json_to_lon(
            r#"{"users":[{"id":1,"payload":{"items":[1,true,null,"x"]}},{"id":2,"payload":{"items":[2,false,null,"y"]}}]}"#,
        )
        .unwrap(),
        json_to_lon(r#"[1,{"a":[2,3]},[4,{"b":5}]]"#).unwrap(),
    ];

    for source in &sources {
        for end in 0..source.len() {
            let truncated = source.get(..end);
            assert!(
                truncated.is_none_or(|prefix| lon_to_json(prefix).is_err()),
                "end {end} of {source}",
            );
        }
    }
}

/// Mirrors the Dart helper: a table is present when a semicolon appears at the
/// array's top level before any closing bracket.
fn has_top_level_table(source: &str) -> bool {
    let mut chars = source.chars();
    if chars.next() != Some('[') {
        return false;
    }

    let mut nested_depth = 0;
    let mut quoted = false;
    let mut escaped = false;
    for c in chars {
        if quoted {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = false;
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            '[' | '{' => nested_depth += 1,
            ']' | '}' => {
                if nested_depth == 0 {
                    return false;
                }
                nested_depth -= 1;
            }
            ';' if nested_depth == 0 => return true,
            _ => {}
        }
    }
    false
}
