//! Serde edge cases beyond `serde.rs`: scalar-type boundaries, nested
//! containers, and equivalence with the `serde_json::Value` codec path.

use serde::{Deserialize, Serialize};
use serde_json::json;

// ------------------------------------------------------- scalar boundaries

#[test]
fn u8_fields_reject_out_of_range_values() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Small {
        v: u8,
    }
    assert_eq!(lon::from_str::<Small>("{v:255}").unwrap(), Small { v: 255 });
    assert!(lon::from_str::<Small>("{v:256}").is_err());
    assert!(lon::from_str::<Small>("{v:300}").is_err());
    assert!(lon::from_str::<Small>("{v:-1}").is_err());
    // u8 also rejects a float lexeme and a string.
    assert!(lon::from_str::<Small>("{v:1.5}").is_err());
    assert!(lon::from_str::<Small>("{v:\"1\"}").is_err());
}

#[test]
fn f32_precision_round_trips() {
    // -0.0 is excluded: like the Dart reference, the value model parses the
    // `-0` lexeme as integer zero, so the sign bit does not survive a
    // string round-trip. See `preserves_extreme_number_lexemes_verbatim` for
    // the text API, which keeps `-0` byte-exact.
    for value in [0.1f32, 1.5e-8, f32::MAX, 123456.75] {
        let encoded = lon::to_string(&value).unwrap();
        let decoded: f32 = lon::from_str(&encoded).unwrap();
        assert_eq!(decoded.to_bits(), value.to_bits(), "f32 {value}");
    }
    // f32 keeps its shortest round-trip spelling distinct from f64 output.
    assert_eq!(lon::to_string(&0.1f32).unwrap(), "0.1");
}

#[test]
fn char_fields_round_trip_as_single_character_strings() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Letter {
        c: char,
    }
    let value = Letter { c: 'x' };
    let encoded = lon::to_string(&value).unwrap();
    assert_eq!(encoded, "{c:x}");
    assert_eq!(lon::from_str::<Letter>(&encoded).unwrap(), value);
    // Multi-byte and supplementary characters still round-trip.
    for c in ['\u{1F600}', '中', '\''] {
        let letter = Letter { c };
        let encoded = lon::to_string(&letter).unwrap();
        assert_eq!(lon::from_str::<Letter>(&encoded).unwrap(), letter);
    }
    // A multi-character or empty string is not a char.
    assert!(lon::from_str::<Letter>("{c:\"ab\"}").is_err());
    assert!(lon::from_str::<Letter>("{c:\"\"}").is_err());
}

#[test]
fn byte_vectors_serialize_as_number_arrays() {
    let bytes: Vec<u8> = vec![0, 1, 127, 255];
    let encoded = lon::to_string(&bytes).unwrap();
    assert_eq!(encoded, "[0,1,127,255]");
    assert_eq!(lon::from_str::<Vec<u8>>(&encoded).unwrap(), bytes);
    // Elements outside the u8 range fail on the way back in.
    assert!(lon::from_str::<Vec<u8>>("[0,256]").is_err());
}

#[test]
fn option_char_round_trips() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Maybe {
        c: Option<char>,
    }
    let some = Maybe { c: Some('z') };
    let none = Maybe { c: None };
    assert_eq!(lon::to_string(&some).unwrap(), "{c:z}");
    assert_eq!(lon::to_string(&none).unwrap(), "{c:null}");
    assert_eq!(lon::from_str::<Maybe>("{c:z}").unwrap(), some);
    assert_eq!(lon::from_str::<Maybe>("{c:null}").unwrap(), none);
}

// ------------------------------------------------------------- nested shapes

#[test]
fn nested_enums_round_trip() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum Inner {
        On,
        Off,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum Outer {
        Flag(Inner),
        Pair { a: Inner, b: Inner },
    }

    for value in [
        Outer::Flag(Inner::On),
        Outer::Pair {
            a: Inner::Off,
            b: Inner::On,
        },
    ] {
        let encoded = lon::to_string(&value).unwrap();
        assert_eq!(lon::from_str::<Outer>(&encoded).unwrap(), value);
    }
    // Unit-variant names colliding with reserved literals are quoted.
    assert_eq!(
        lon::to_string(&Outer::Flag(Inner::On)).unwrap(),
        "{Flag:\"On\"}"
    );
    assert_eq!(
        lon::to_string(&Outer::Pair {
            a: Inner::Off,
            b: Inner::On
        })
        .unwrap(),
        "{Pair:{a:\"Off\" b:\"On\"}}"
    );
}

#[test]
fn nested_struct_vectors_hit_nested_table_cells() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Cell {
        s: String,
        q: u32,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Row {
        id: u32,
        items: Vec<Cell>,
    }

    let rows = vec![
        Row {
            id: 1,
            items: vec![
                Cell {
                    s: "x".into(),
                    q: 2,
                },
                Cell {
                    s: "y".into(),
                    q: 1,
                },
            ],
        },
        Row {
            id: 2,
            items: vec![
                Cell {
                    s: "w".into(),
                    q: 4,
                },
                Cell {
                    s: "v".into(),
                    q: 5,
                },
            ],
        },
    ];
    let encoded = lon::to_string(&rows).unwrap();
    assert_eq!(encoded, "[id items;1 [s q;x 2;\"y\" 1];2 [s q;w 4;v 5]]");
    assert_eq!(lon::from_str::<Vec<Row>>(&encoded).unwrap(), rows);
}

#[test]
fn vec_of_vec_of_struct_expands_into_nested_tables() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Item {
        k: u32,
        v: u32,
    }

    let grid = vec![
        vec![Item { k: 1, v: 2 }, Item { k: 3, v: 4 }],
        vec![Item { k: 5, v: 6 }, Item { k: 7, v: 8 }],
    ];
    let encoded = lon::to_string(&grid).unwrap();
    // The outer array is a record table whose cells are record tables.
    assert_eq!(lon::from_str::<Vec<Vec<Item>>>(&encoded).unwrap(), grid);
    // Round-trip through the Value codec to confirm table-cell parsing.
    let value = lon::LonCodec::new().decode(&encoded).unwrap();
    assert_eq!(
        value,
        json!([
            [{"k": 1, "v": 2}, {"k": 3, "v": 4}],
            [{"k": 5, "v": 6}, {"k": 7, "v": 8}]
        ])
    );
}

// ----------------------------------------------------------- untagged enums

#[test]
fn untagged_enum_selects_among_three_branches() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    #[serde(untagged)]
    enum Flex {
        Int(i64),
        Text(String),
        List(Vec<i64>),
    }

    assert_eq!(lon::from_str::<Flex>("42").unwrap(), Flex::Int(42));
    assert_eq!(
        lon::from_str::<Flex>("hello").unwrap(),
        Flex::Text("hello".into())
    );
    assert_eq!(
        lon::from_str::<Flex>("[1 2]").unwrap(),
        Flex::List(vec![1, 2])
    );
    assert_eq!(lon::to_string(&Flex::Int(42)).unwrap(), "42");
    assert_eq!(lon::to_string(&Flex::Text("hi".into())).unwrap(), "hi");
    assert_eq!(lon::to_string(&Flex::List(vec![1, 2])).unwrap(), "[1,2]");
}

// ------------------------------------------------- codec/value equivalence

#[test]
fn serde_decode_matches_the_codec_value_path() {
    for source in [
        "{users:[id name;1 Ada;2 Bob]}",
        "{a:{b:[1,{c:null}]} d:[x \"y\"]}",
        "[id payload{a{b}};1 2;3 4]",
        "[1,[2,[3]],{k:v}]",
        "null",
        "\"quoted token\"",
    ] {
        let via_serde: serde_json::Value = lon::from_str(source).unwrap();
        let via_codec = lon::LonCodec::new().decode(source).unwrap();
        assert_eq!(via_serde, via_codec, "{source}");
    }
}
