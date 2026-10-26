//! Serde integration tests: derive structs/enums directly against LON.

use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct User {
    id: u32,
    name: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Config {
    version: String,
    debug: bool,
    retries: Option<u8>,
    note: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Outer {
    config: Config,
    users: Vec<User>,
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Status {
    Active,
    Blocked,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Shape {
    Point,
    Radius(f64),
    Rect { w: u32, h: u32 },
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(untagged)]
enum IntOrText {
    Int(i64),
    Text(String),
}

#[test]
fn struct_vector_hits_record_table() {
    let users = vec![
        User {
            id: 1,
            name: "Ada".into(),
        },
        User {
            id: 2,
            name: "Bob".into(),
        },
    ];
    let encoded = lon::to_string(&users).unwrap();
    assert_eq!(encoded, "[id name;1 Ada;2 Bob]");
    assert_eq!(lon::from_str::<Vec<User>>(&encoded).unwrap(), users);
}

#[test]
fn nested_struct_round_trip() {
    let value = Outer {
        config: Config {
            version: "1.0".into(),
            debug: true,
            retries: Some(3),
            note: None,
        },
        users: vec![
            User {
                id: 7,
                name: "Grace Hopper".into(),
            },
            User {
                id: 8,
                name: "Lin".into(),
            },
        ],
        tags: vec!["alpha".into(), "beta".into()],
    };
    let encoded = lon::to_string(&value).unwrap();
    assert_eq!(
        encoded,
        "{config:{version:\"1.0\" debug:true retries:3 note:null} users:[id name;7 \"Grace Hopper\";8 Lin] tags:[alpha beta]}"
    );
    assert_eq!(lon::from_str::<Outer>(&encoded).unwrap(), value);
}

#[test]
fn option_none_serializes_as_null() {
    let value = Config {
        version: "2".into(),
        debug: false,
        retries: None,
        note: None,
    };
    let encoded = lon::to_string(&value).unwrap();
    assert_eq!(
        encoded,
        "{version:\"2\" debug:false retries:null note:null}"
    );
    assert_eq!(lon::from_str::<Config>(&encoded).unwrap(), value);
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Wrapper(u64);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Empty {}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Tuples(Point2, Point2);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Point2 {
    x: i32,
    y: i32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct UnitLike;

#[test]
fn newtype_unit_empty_and_tuple_structs() {
    assert_eq!(lon::to_string(&Wrapper(9)).unwrap(), "9");
    assert_eq!(lon::from_str::<Wrapper>("9").unwrap(), Wrapper(9));

    assert_eq!(lon::to_string(&Empty {}).unwrap(), "{}");
    assert_eq!(lon::from_str::<Empty>("{}").unwrap(), Empty {});

    assert_eq!(lon::to_string(&UnitLike).unwrap(), "null");
    assert_eq!(lon::from_str::<UnitLike>("null").unwrap(), UnitLike);

    let pair = Tuples(Point2 { x: 1, y: 2 }, Point2 { x: 3, y: 4 });
    let encoded = lon::to_string(&pair).unwrap();
    // Uniform structs inside the tuple array hit the record-table form.
    assert_eq!(encoded, "[x y;1 2;3 4]");
    assert_eq!(lon::from_str::<Tuples>(&encoded).unwrap(), pair);
}

#[test]
fn enum_variants_round_trip() {
    assert_eq!(lon::to_string(&Status::Active).unwrap(), "Active");
    assert_eq!(lon::from_str::<Status>("Blocked").unwrap(), Status::Blocked);

    assert_eq!(lon::to_string(&Shape::Radius(2.5)).unwrap(), "{Radius:2.5}");
    assert_eq!(
        lon::from_str::<Shape>("{Radius:2.5}").unwrap(),
        Shape::Radius(2.5)
    );

    assert_eq!(
        lon::to_string(&Shape::Rect { w: 3, h: 4 }).unwrap(),
        "{Rect:{w:3 h:4}}"
    );
    assert_eq!(
        lon::from_str::<Shape>("{Rect:{w:3 h:4}}").unwrap(),
        Shape::Rect { w: 3, h: 4 }
    );
    // The null form of an externally tagged unit variant also decodes.
    assert_eq!(
        lon::from_str::<Shape>("{Point:null}").unwrap(),
        Shape::Point
    );
}

#[test]
fn untagged_enum_uses_deserialize_any() {
    assert_eq!(
        lon::from_str::<IntOrText>("42").unwrap(),
        IntOrText::Int(42)
    );
    assert_eq!(
        lon::from_str::<IntOrText>("hello").unwrap(),
        IntOrText::Text("hello".into())
    );
    assert_eq!(lon::to_string(&IntOrText::Int(42)).unwrap(), "42");
    assert_eq!(
        lon::to_string(&IntOrText::Text("hello".into())).unwrap(),
        "hello"
    );
}

#[test]
fn integer_extremes_round_trip() {
    assert_eq!(lon::to_string(&i64::MIN).unwrap(), "-9223372036854775808");
    assert_eq!(
        lon::from_str::<i64>("-9223372036854775808").unwrap(),
        i64::MIN
    );
    assert_eq!(lon::to_string(&i64::MAX).unwrap(), "9223372036854775807");
    assert_eq!(lon::to_string(&u64::MAX).unwrap(), "18446744073709551615");
    assert_eq!(
        lon::from_str::<u64>("18446744073709551615").unwrap(),
        u64::MAX
    );
    assert_eq!(
        lon::from_str::<i128>("170141183460469231731687303715884105727").unwrap(),
        i128::MAX
    );
}

#[test]
fn float_precision_round_trip() {
    let values = [0.1f64 + 0.2f64, 1e300, -0.0, 1.5e-8, 123456.789];
    for value in values {
        let encoded = lon::to_string(&value).unwrap();
        assert_eq!(lon::from_str::<f64>(&encoded).unwrap(), value);
    }
    assert_eq!(lon::to_string(&0.1f32).unwrap(), "0.1");
}

#[test]
fn non_finite_floats_are_rejected() {
    assert!(lon::to_string(&f64::NAN).is_err());
    assert!(lon::to_string(&f64::INFINITY).is_err());
    assert!(lon::to_string(&f32::NEG_INFINITY).is_err());
}

#[test]
fn i128_u128_out_of_range_are_rejected() {
    let big = i128::from(i64::MAX) + 1;
    assert!(lon::to_string(&big).is_err());
    let big = u128::from(u64::MAX) + 1;
    assert!(lon::to_string(&big).is_err());
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Strict {
    only: u8,
}

#[test]
fn error_paths() {
    // Truncated document.
    assert!(lon::from_str::<Vec<User>>("[id name;1 Ada").is_err());
    // Type mismatch: string where a number is required.
    let error = lon::from_str::<Strict>("{only:Ada}").unwrap_err();
    assert!(error.message().contains("expected"));
    // Trailing content.
    assert!(lon::from_str::<Strict>("{only:1} extra").is_err());
    // Out-of-range integer for u8.
    assert!(lon::from_str::<Strict>("{only:300}").is_err());
    // Unknown fields are ignored by default.
    assert_eq!(
        lon::from_str::<Strict>("{only:1 other:2}").unwrap(),
        Strict { only: 1 }
    );
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
struct DenyUnknown {
    only: u8,
}

#[test]
fn deny_unknown_fields() {
    assert_eq!(
        lon::from_str::<DenyUnknown>("{only:1}").unwrap(),
        DenyUnknown { only: 1 }
    );
    assert!(lon::from_str::<DenyUnknown>("{only:1 other:2}").is_err());
}

#[test]
fn duplicate_keys_keep_last_value() {
    // Struct visitors reject duplicates (serde semantics); maps keep the
    // last value for a repeated key.
    let decoded: std::collections::BTreeMap<String, u8> = lon::from_str("{a:1 a:2}").unwrap();
    assert_eq!(decoded.get("a"), Some(&2));
}

#[test]
fn empty_containers_and_empty_string_key() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct WithEmpty {
        empty: Vec<String>,
        #[serde(rename = "")]
        blank: u8,
    }
    let value = WithEmpty {
        empty: vec![],
        blank: 5,
    };
    let encoded = lon::to_string(&value).unwrap();
    assert_eq!(encoded, "{empty:[] \"\":5}");
    assert_eq!(lon::from_str::<WithEmpty>(&encoded).unwrap(), value);
}

#[test]
fn pretty_output_round_trips() {
    let users = vec![
        User {
            id: 1,
            name: "Ada".into(),
        },
        User {
            id: 2,
            name: "Bob".into(),
        },
    ];
    let pretty = lon::to_string_pretty(&users).unwrap();
    assert_eq!(pretty, "[\n  id name;\n  1 Ada;\n  2 Bob\n]");
    assert_eq!(lon::from_str::<Vec<User>>(&pretty).unwrap(), users);
    // The compact and pretty forms decode to identical values.
    assert_eq!(
        lon::to_string(&users).unwrap(),
        lon::json_to_lon(&serde_json::to_string(&users).unwrap()).unwrap()
    );
}

#[test]
fn codec_options_apply_to_serde_path() {
    let users = vec![
        User {
            id: 1,
            name: "Ada".into(),
        },
        User {
            id: 2,
            name: "Bob".into(),
        },
    ];
    let without_tables = lon::LonCodec::new()
        .use_tables(false)
        .to_string(&users)
        .unwrap();
    assert_eq!(without_tables, "[{id:1 name:Ada},{id:2 name:Bob}]");

    let depth_limited = lon::LonCodec::new().max_depth(0).to_string(&users);
    assert!(depth_limited.is_err());

    let element_limited = lon::LonCodec::new()
        .max_expanded_elements(3)
        .to_string(&users);
    assert!(element_limited.is_err());
}

#[test]
fn maps_and_integer_keys() {
    let sorted: std::collections::BTreeMap<u32, &str> = [(2, "b"), (1, "a")].into_iter().collect();
    let encoded = lon::to_string(&sorted).unwrap();
    assert_eq!(encoded, "{1:a 2:b}");
    let decoded: std::collections::BTreeMap<u32, String> = lon::from_str(&encoded).unwrap();
    let expected: std::collections::BTreeMap<u32, String> =
        [(1, "a".to_string()), (2, "b".to_string())]
            .into_iter()
            .collect();
    assert_eq!(decoded, expected);
}

#[test]
fn serde_json_value_interop() {
    let source = "{users:[id name;1 Ada;2 Bob]}";
    let via_value: serde_json::Value = lon::from_str(source).unwrap();
    assert_eq!(
        via_value,
        json!({"users": [{"id": 1, "name": "Ada"}, {"id": 2, "name": "Bob"}]})
    );
    let via_typed: serde_json::Value = lon::LonCodec::new().decode(source).unwrap();
    assert_eq!(via_value, via_typed);
}
