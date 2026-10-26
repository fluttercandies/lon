//! CLI behavior tests mirroring the Dart `bin/lon.dart` process tests.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::json;

fn run_lon(args: &[&str], input: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lon"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn lon binary");
    child
        .stdin
        .as_mut()
        .expect("piped stdin")
        .write_all(input)
        .expect("write stdin");
    child.wait_with_output().expect("wait for lon")
}

fn code(output: &std::process::Output) -> i32 {
    output.status.code().expect("normal exit")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("valid UTF-8 stdout")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("valid UTF-8 stderr")
}

#[test]
fn round_trips_split_utf_8_byte_streams() {
    let value = json!({"空 白;\"[]{}": "line one\r\nline two,;[]{}\"\\😀\u{202E}"});
    let input = serde_json::to_string(&value).unwrap();

    let output = run_lon(&["encode"], input.as_bytes());
    assert_eq!(code(&output), 0);
    assert!(stderr(&output).is_empty());
    let text = stdout(&output);
    assert!(text.ends_with('\n'), "document is followed by one newline");
    let payload = &text[..text.len() - 1];
    assert!(
        !payload.contains('\n'),
        "the payload itself stays on one line"
    );

    let decoded = run_lon(&["decode"], payload.as_bytes());
    assert_eq!(code(&decoded), 0);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stdout(&decoded)).unwrap(),
        value,
    );
}

#[test]
fn rejects_malformed_utf_8_byte_streams() {
    let output = run_lon(&["decode"], &[0x22, 0xC3, 0x28, 0x22]);
    assert_eq!(code(&output), 65);
    assert!(output.stdout.is_empty());
    let diagnostic = stderr(&output);
    assert!(diagnostic.contains("Invalid input"), "{diagnostic}");
    assert!(!diagnostic.contains('\u{FFFD}'), "{diagnostic}");
}

#[test]
fn does_not_echo_untrusted_input_in_diagnostics() {
    let output = run_lon(&["decode"], b"ok \x1B]0;PWNED\x07");
    assert_eq!(code(&output), 65);
    assert!(output.stdout.is_empty());
    let diagnostic = stderr(&output);
    assert!(
        diagnostic.contains("Unexpected trailing content"),
        "{diagnostic}"
    );
    assert!(!diagnostic.contains("PWNED"));
    assert!(!diagnostic.contains('\x1B'));
    assert!(!diagnostic.contains('\x07'));
}

#[test]
fn rejects_invalid_usage() {
    assert_eq!(code(&run_lon(&[], b"")), 64);
    assert_eq!(code(&run_lon(&["bogus"], b"")), 64);
    assert_eq!(code(&run_lon(&["encode", "a", "b"], b"")), 64);
    assert_eq!(code(&run_lon(&["encode", "--bogus"], b"")), 64);
}

#[test]
fn encodes_and_decodes_with_pretty() {
    let encoded = run_lon(
        &["encode", "--pretty"],
        br#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}"#,
    );
    assert_eq!(code(&encoded), 0);
    assert!(stderr(&encoded).is_empty());
    let text = stdout(&encoded);
    assert!(text.ends_with('\n'));
    let payload = &text[..text.len() - 1];
    assert_eq!(
        payload,
        "{\n  users: [\n    id name;\n    1 Ada;\n    2 Bob\n  ]\n}",
    );

    let decoded = run_lon(&["decode", "--pretty"], payload.as_bytes());
    assert_eq!(code(&decoded), 0);
    assert_eq!(
        stdout(&decoded),
        "{\n  \"users\": [\n    {\n      \"id\": 1,\n      \"name\": \"Ada\"\n    },\n    {\n      \"id\": 2,\n      \"name\": \"Bob\"\n    }\n  ]\n}\n",
    );

    // The same document without --pretty still round-trips through --pretty.
    let compact = run_lon(&["decode"], payload.as_bytes());
    assert_eq!(code(&compact), 0);
    assert_eq!(
        stdout(&compact),
        "{\"users\":[{\"id\":1,\"name\":\"Ada\"},{\"id\":2,\"name\":\"Bob\"}]}\n",
    );
}

#[test]
fn reports_missing_files() {
    let output = run_lon(&["encode", "/nonexistent/lon-cli-test.json"], b"");
    assert_eq!(code(&output), 66);
    assert!(output.stdout.is_empty());
}

#[test]
fn encodes_and_decodes_files() {
    let path = std::env::temp_dir().join(format!("lon-cli-test-{}.json", std::process::id()));
    std::fs::write(
        &path,
        b"{\"users\":[{\"id\":1,\"name\":\"Ada\"},{\"id\":2,\"name\":\"Bob\"}]}",
    )
    .unwrap();

    let encoded = run_lon(&["encode", path.to_str().unwrap()], b"");
    std::fs::remove_file(&path).unwrap();
    assert_eq!(code(&encoded), 0);
    assert_eq!(stdout(&encoded), "{users:[id name;1 Ada;2 Bob]}\n");

    std::fs::write(&path, b"{users:[id name;1 Ada;2 Bob]}").unwrap();
    let decoded = run_lon(&["decode", path.to_str().unwrap()], b"");
    std::fs::remove_file(&path).unwrap();
    assert_eq!(code(&decoded), 0);
    assert_eq!(
        stdout(&decoded),
        "{\"users\":[{\"id\":1,\"name\":\"Ada\"},{\"id\":2,\"name\":\"Bob\"}]}\n",
    );
}

#[test]
fn rejects_single_row_tables_and_truncated_input_with_65() {
    let output = run_lon(&["decode"], b"{users:[id name;1 Ada]}");
    assert_eq!(code(&output), 65);

    let output = run_lon(&["decode"], b"{users:[id name;1 Ada;2 Bob");
    assert_eq!(code(&output), 65);
}

#[test]
fn check_validates_lon_without_output() {
    let source = b"{users:[id name;1 Ada;2 Bob]}";

    let from_stdin = run_lon(&["check"], source);
    assert_eq!(code(&from_stdin), 0);
    assert!(from_stdin.stdout.is_empty());
    assert!(from_stdin.stderr.is_empty());

    let path = std::env::temp_dir().join(format!("lon-cli-check-{}.lon", std::process::id()));
    std::fs::write(&path, source).unwrap();
    let from_file = run_lon(&["check", path.to_str().unwrap()], b"");
    std::fs::remove_file(&path).unwrap();
    assert_eq!(code(&from_file), 0);
    assert!(from_file.stdout.is_empty());
}

#[test]
fn check_rejects_invalid_input_with_65() {
    for input in [
        &b"{a:1"[..],
        b"{a:1 b:[x y]}", // 'y' is a reserved literal
        b"",
        b"\xEF\xBB\xBF{a:1}",
        b"\xFF\xFE\"",
    ] {
        let output = run_lon(&["check"], input);
        assert_eq!(code(&output), 65, "{input:?}");
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn check_reports_missing_files_and_rejects_extra_arguments() {
    let missing = run_lon(&["check", "/nonexistent/lon-cli-check-test.lon"], b"");
    assert_eq!(code(&missing), 66);
    assert!(missing.stdout.is_empty());

    let extra = run_lon(&["check", "a", "b"], b"");
    assert_eq!(code(&extra), 64);
    assert!(extra.stdout.is_empty());
}
