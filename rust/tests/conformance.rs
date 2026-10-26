//! Cross-implementation conformance fixtures from `conformance/v1.json`.

use lon::{json_to_lon, lon_to_json};

#[test]
fn v1_conformance_cases() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../conformance/v1.json");
    let source = std::fs::read_to_string(path).expect("conformance fixtures exist");
    let cases: Vec<serde_json::Value> = serde_json::from_str(&source).unwrap();

    assert!(!cases.is_empty());
    for case in &cases {
        let name = case["name"].as_str().unwrap();
        let json = case["json"].as_str().unwrap();
        let lon = case["lon"].as_str().unwrap();
        let canonical_json = case["canonicalJson"].as_str().unwrap();

        assert_eq!(json_to_lon(json).unwrap(), lon, "{name}: json -> lon");
        assert_eq!(
            lon_to_json(lon).unwrap(),
            canonical_json,
            "{name}: lon -> json"
        );
        assert_eq!(
            json_to_lon(&lon_to_json(lon).unwrap()).unwrap(),
            lon,
            "{name}: canonical LON is idempotent",
        );
    }
}
