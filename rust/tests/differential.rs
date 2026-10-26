//! Differential test against the Dart reference implementation.
//!
//! Runs `tests/diff_harness.dart` through the sibling `dart/` package and
//! asserts that this crate accepts and rejects exactly the same inputs and
//! produces byte-identical outputs. Skipped when no Dart SDK (or unresolved
//! Dart package) is available, for example on a Rust-only CI runner.

use std::path::Path;
use std::process::Command;

use lon::{json_to_lon, lon_to_json};

#[test]
fn matches_the_dart_reference_codec() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let dart_dir = root.join("dart");
    let package_config = dart_dir.join(".dart_tool/package_config.json");
    let harness = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/diff_harness.dart");
    if Command::new("dart").arg("--version").output().is_err()
        || !package_config.exists()
        || !harness.exists()
    {
        eprintln!("skipping: dart SDK or Dart package not available");
        return;
    }

    let output = Command::new("dart")
        .arg("--packages=.dart_tool/package_config.json")
        .arg(dart_dir.join("../rust/tests/diff_harness.dart"))
        .current_dir(&dart_dir)
        .output()
        .expect("run dart harness");
    assert!(
        output.status.success(),
        "dart harness failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let results: Vec<serde_json::Value> =
        serde_json::from_slice(&output.stdout).expect("harness emits one JSON array");
    assert!(
        results.len() > 100,
        "expected a full battery, got {}",
        results.len()
    );

    for case in &results {
        let kind = case["kind"].as_str().unwrap();
        let input = case["input"].as_str().unwrap();
        let dart_ok = case["ok"].as_bool().unwrap();
        let dart_output = case["output"].as_str();

        let rust = if kind == "jsonToLon" {
            json_to_lon(input)
        } else {
            lon_to_json(input)
        };
        match (dart_ok, rust) {
            (true, Ok(rust_output)) => assert_eq!(
                Some(rust_output.as_str()),
                dart_output,
                "{kind} of {input:?}",
            ),
            (false, Err(_)) => {}
            (true, Err(error)) => {
                panic!("{kind} of {input:?}: Dart accepted, Rust rejected: {error}")
            }
            (false, Ok(rust_output)) => {
                panic!("{kind} of {input:?}: Dart rejected, Rust accepted: {rust_output}",)
            }
        }
    }
}
