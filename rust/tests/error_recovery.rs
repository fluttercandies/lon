//! Ports of `dart/test/error_recovery_test.dart`: rejection coverage for
//! malformed table rows, unterminated structures, and truncated input.

use lon::{json_to_lon, lon_to_json, ErrorKind, LonError};

fn expect_format_error<T: std::fmt::Debug>(result: Result<T, LonError>, context: &str) {
    let Err(error) = result else {
        panic!("expected a format error for {context:?}, got {result:?}")
    };
    assert_eq!(error.kind(), ErrorKind::Format, "context: {context:?}");
}

#[test]
fn rejects_table_row_width_mismatches() {
    for source in [
        // Fewer cells than the schema declares.
        "[a b;1 2;3]",
        "[a b;1;2 3]",
        "[a b c;1 2 3;4 5]",
        "[a{x y} b;1 2;3 4]",
        // More cells than the schema declares.
        "[a b;1 2;3 4 5]",
        "[a b;1 2 3;4 5]",
        "[a{x y} b;1 2 3 4;5 6 7]",
        // Empty row or double semicolons.
        "[a b;;1 2;3 4]",
        "[a b;1 2;;3 4]",
        "[a b;1 2;3 4;]",
        // A nested table-shaped row in a schema row position.
        "[a b;[1 2];[3 4]]",
        "[a b;{x:1} {y:2}]",
    ] {
        expect_format_error(lon_to_json(source), source);
    }
}

#[test]
fn rejects_unterminated_structures() {
    for source in [
        // A table header run hits the closing bracket: the schema ends without
        // the ';' terminator, and one row is never enough.
        "[a b;]",
        "[a b;1 2;]",
        "[a{x y} b;]",
        // Unterminated schema or row.
        "[a b;1 2",
        "[a b;1 2;3 4",
        "[a{x y} b;1 2 3",
        // Unterminated containers.
        "{a:1",
        "{a:1 b:2",
        "[1,2",
        "[[1],[2",
        "{a:{b:{c:1}}",
        // Missing separators between adjacent values.
        "[1 2}3]",
        "{a:1,b:2}",
        // Stray semicolons outside tables.
        "{a:1;b:2}",
        "[1;2]",
    ] {
        expect_format_error(lon_to_json(source), source);
    }
}

#[test]
fn rejects_malformed_quoted_strings() {
    for source in [
        // Unterminated quote at various positions.
        "{a:\"unterminated",
        "[a b;\"x\" 1;\"y 2]",
        "\"unterminated",
        "{\"key\":",
        // Invalid escape sequences.
        r#"{a:"bad\x"}"#,
        r#"[a b;"x\x" 1;"y" 2]"#,
        r#""bad\q""#,
        r#""\u12""#,
        r#""\uzzzz""#,
        // A dangling backslash at end of input.
        "\"trailing\\",
    ] {
        expect_format_error(lon_to_json(source), source);
    }
    // Unescaped control characters.
    expect_format_error(lon_to_json("\"raw\nnewline\""), "raw newline");
    expect_format_error(lon_to_json("\"raw\u{0}control\""), "raw control");
}

#[test]
fn rejects_every_prefix_of_a_deep_container() {
    let encoded = json_to_lon(
        r#"{"a":{"b":{"c":{"d":{"e":[1,{"f":{"g":2}}],"h":[{"i":1,"j":2},{"i":3,"j":4}]}}}}}"#,
    )
    .unwrap();
    for end in 0..encoded.len() {
        expect_format_error(
            lon_to_json(&encoded[..end]),
            &format!("prefix of length {end} of {encoded}"),
        );
    }
    // Sanity-check the source itself parses.
    assert!(lon_to_json(&encoded).is_ok());
}

#[test]
fn rejects_every_prefix_of_a_nested_table_header() {
    let source = "[a{b{c{x}}} y;0 1 2 3;4 5 6 7]";
    for end in 0..source.len() {
        expect_format_error(
            lon_to_json(&source[..end]),
            &format!("prefix of length {end} of {source}"),
        );
    }
}
