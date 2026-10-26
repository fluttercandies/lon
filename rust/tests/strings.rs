//! Ports of the deterministic string-domain tests in
//! `dart/test/string_round_trip_test.dart`.

use lon::{json_to_lon, lon_to_json, LonCodec};
use serde_json::{json, Value};

fn expect_value_and_json_round_trip(value: &Value) {
    let codec = LonCodec::new();
    let encoded = codec.encode(value).unwrap();
    assert_eq!(codec.decode(&encoded).unwrap(), *value);

    let json = serde_json::to_string(value).unwrap();
    let encoded_json = json_to_lon(&json).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&lon_to_json(&encoded_json).unwrap()).unwrap(),
        *value,
    );
    assert_eq!(
        json_to_lon(&lon_to_json(&encoded_json).unwrap()).unwrap(),
        encoded_json
    );
}

#[test]
fn preserves_complex_embedded_whitespace_exactly() {
    let value = json!("abc 12 34 \nxxx\nb \txx");
    assert_eq!(
        LonCodec::new().encode(&value).unwrap(),
        r#""abc 12 34 \nxxx\nb \txx""#,
    );
    expect_value_and_json_round_trip(&value);
}

#[test]
fn preserves_empty_leading_middle_and_trailing_whitespace() {
    let values = json!([
        "",
        " ",
        "  leading",
        "middle  space",
        "trailing  ",
        "\tleading tab",
        "line one\nline two",
        "carriage\rreturn",
        "form\u{c}feed",
        "back\u{8}space",
        "mixed \n\r\t whitespace",
    ]);
    expect_value_and_json_round_trip(&values);
}

#[test]
fn preserves_every_c0_and_c1_control_code_unit() {
    let values = Value::Array(
        (0..=0x9Fu32)
            .filter_map(char::from_u32)
            .map(|c| Value::String(c.to_string()))
            .collect(),
    );

    let encoded = LonCodec::new().encode(&values).unwrap();
    assert_eq!(LonCodec::new().decode(&encoded).unwrap(), values);
    expect_value_and_json_round_trip(&values);

    for c in encoded.chars() {
        let code = c as u32;
        assert!(
            !(code < 0x20 || (0x7F..=0x9F).contains(&code)),
            "raw control U+{code:04X}",
        );
    }
}

#[test]
fn preserves_punctuation_delimiters_and_json_escapes() {
    let values = json!([
        "\"",
        "\\",
        "/",
        ":",
        ",",
        ";",
        "[",
        "]",
        "{",
        "}",
        "(",
        ")",
        "|",
        "'",
        "quote\"slash\\solidus/",
        "https://example.com/a?x=1&y=2#part",
        "[not a container]",
        "{not:an object}",
    ]);
    expect_value_and_json_round_trip(&values);
}

const GRAMMAR_CONFLICT_ATOMS: [char; 17] = [
    ' ', '\t', '\r', '\n', '"', '\\', ',', ';', ':', '[', ']', '{', '}', '(', ')', '|', '\'',
];

fn grammar_conflict_strings() -> Vec<Value> {
    let mut values = vec![Value::String(String::new())];
    for first in GRAMMAR_CONFLICT_ATOMS {
        for second in GRAMMAR_CONFLICT_ATOMS {
            for third in GRAMMAR_CONFLICT_ATOMS {
                let text: String = [first, second, third].iter().collect();
                values.push(Value::String(text));
            }
        }
    }
    values
}

#[test]
fn preserves_exhaustive_short_grammar_conflict_combinations() {
    let mut values = grammar_conflict_strings();
    values.extend([
        json!("key:value;next,[array],{object}"),
        json!(r#"quote\"slash\\comma,semicolon;"#),
        json!("// comment-like"),
        json!("/* block-comment-like */"),
        json!("# yaml-comment-like"),
        json!("&anchor *alias !tag"),
        json!("```json\n{\"x\":\"</script>\"}\n```"),
        json!(r#"${template} %0A %0D ?x=1&y=2#fragment"#),
        json!("<script>alert(\"x\")</script>"),
    ]);
    expect_value_and_json_round_trip(&Value::Array(values));
}

#[test]
fn preserves_delimiter_heavy_table_schema_keys_and_cells() {
    let value = json!([
        {"a;b": "x;y", "q\"x": "[one]", "line\nkey": "{one}"},
        {"a;b": "x,y", "q\"x": "[two]", "line\nkey": "{two}"},
        {"a;b": "x:y", "q\"x": "[three]", "line\nkey": "{three}"},
    ]);

    let encoded = LonCodec::new().encode(&value).unwrap();
    assert!(
        encoded.starts_with(r#"["a;b" "q\"x" "line\nkey";"#),
        "{encoded}"
    );
    expect_value_and_json_round_trip(&value);
}

#[test]
fn round_trips_every_utf_16_code_unit() {
    for unit in 0..=0xFFFFu32 {
        let json = format!("\"\\u{unit:04x}\"");
        let encoded = json_to_lon(&json).unwrap_or_else(|error| panic!("{unit:#06x}: {error}"));
        let decoded = lon_to_json(&encoded).unwrap_or_else(|error| panic!("{unit:#06x}: {error}"));
        let reencoded =
            json_to_lon(&decoded).unwrap_or_else(|error| panic!("{unit:#06x}: {error}"));
        assert_eq!(reencoded, encoded, "unit U+{unit:04X}");
    }
}

#[test]
fn round_trips_every_utf_16_code_unit_as_an_object_key() {
    let mut json = String::from("{");
    for unit in 0..=0xFFFFu32 {
        json.push_str(&format!("\"\\u{unit:04x}\":{unit},"));
    }
    json.push_str("\"end\":0}");

    let encoded = json_to_lon(&json).unwrap();
    let decoded = lon_to_json(&encoded).unwrap();
    assert_eq!(json_to_lon(&decoded).unwrap(), encoded);
}

#[test]
fn preserves_json_adjacent_and_numeric_looking_strings() {
    let values = json!([
        "null",
        "NULL",
        "true",
        "False",
        "undefined",
        "None",
        "nil",
        "NaN",
        "+Infinity",
        "-inf",
        ".NaN",
        "+.inf",
        "yes",
        "NO",
        "on",
        "Off",
        "y",
        "N",
        "~",
        "01",
        "+1",
        ".5",
        "1.",
        "1e",
        "1e+",
        "0x10",
        "0b101",
        "0o77",
        "1_000",
        "2026-08-02",
        "123abc",
        "table",
    ]);
    expect_value_and_json_round_trip(&values);

    let encoded = LonCodec::new().encode(&values).unwrap();
    for item in values.as_array().unwrap() {
        let text = item.as_str().unwrap();
        if text == "2026-08-02" || text == "123abc" || text == "table" {
            continue;
        }
        assert!(
            encoded.contains(&serde_json::to_string(item).unwrap()),
            "{text} stays quoted",
        );
    }
    assert!(!encoded.contains("\"table\""));
}

#[test]
fn preserves_multilingual_text_without_unicode_normalization() {
    let values = json!([
        "English: Hello world",
        "Español: ¿Qué tal?",
        "Français : Bonjour le monde",
        "Deutsch: Grüße aus Köln",
        "Português: Olá mundo",
        "Русский: Привет, мир",
        "Українська: Привіт, світе",
        "Ελληνικά: Γειά σου κόσμε",
        "简体中文：你好，世界",
        "繁體中文：你好，世界",
        "日本語：こんにちは世界",
        "한국어: 안녕하세요 세계",
        "العربية: مرحبًا بالعالم",
        "עברית: שלום עולם",
        "فارسی: می‌روم",
        "हिन्दी: नमस्ते दुनिया",
        "বাংলা: নমস্কার বিশ্ব",
        "தமிழ்: வணக்கம் உலகம்",
        "తెలుగు: నమస్కారం ప్రపంచం",
        "ไทย: สวัสดีชาวโลก",
        "Tiếng Việt: Xin chào thế giới",
        "ქართული: გამარჯობა მსოფლიო",
        "Հայերեն: Բարև աշխարհ",
        "አማርኛ: ሰላም ዓለም",
        "ᐃᓄᒃᑎᑐᑦ: ᐊᐃ",
        "é",
        "é",
        "👩‍💻",
        "👨‍👩‍👧‍👦",
        "🇨🇳🇯🇵🇰🇷",
    ]);
    expect_value_and_json_round_trip(&values);
    let items = values.as_array().unwrap();
    assert_ne!(items[25], items[26], "NFD and NFC forms stay distinct");
}

#[test]
fn preserves_unicode_whitespace_and_directional_formatting() {
    let values = json!([
        "a\u{85}b",
        "a\u{a0}b",
        "a\u{1680}b",
        "a\u{2000}b",
        "a\u{200A}b",
        "a\u{2028}b",
        "a\u{2029}b",
        "a\u{202F}b",
        "a\u{205F}b",
        "a\u{3000}b",
        "a\u{200B}b",
        "a\u{ad}b",
        "a\u{34F}b",
        "a\u{61C}b",
        "a\u{180E}b",
        "a\u{FDD0}b",
        "a\u{FFFE}b",
        "a\u{200E}b",
        "a\u{200F}b",
        "a\u{202A}b",
        "a\u{202E}b",
        "a\u{2060}b",
        "a\u{2066}b",
        "a\u{2069}b",
        "a\u{FEFF}b",
    ]);
    expect_value_and_json_round_trip(&values);

    let encoded = LonCodec::new().encode(&values).unwrap();
    for escape in [
        r"\u0085", r"\u2028", r"\u2029", r"\u200b", r"\u00ad", r"\u034f", r"\u061c", r"\u180e",
        r"\ufdd0", r"\ufffe", r"\u202a", r"\u202e", r"\u2060", r"\u2066", r"\ufeff",
    ] {
        assert!(encoded.contains(escape), "{escape}");
    }
}

#[test]
fn preserves_lone_surrogates_and_supplementary_characters() {
    const JSON: &str = r#"["\ud800","\udfff","\ud83d\ude00"]"#;
    let encoded = json_to_lon(JSON).unwrap();
    assert_eq!(
        lon_to_json(&encoded).unwrap(),
        r#"["\ud800","\udfff","😀"]"#
    );
    assert_eq!(
        json_to_lon(&lon_to_json(&encoded).unwrap()).unwrap(),
        encoded
    );

    const SUPPLEMENTARY: &str = r#"["\udb40\udc01","\udbff\udfff"]"#;
    let supplementary = json_to_lon(SUPPLEMENTARY).unwrap();
    assert!(supplementary.contains(r"\udb40\udc01"));
    assert!(supplementary.contains(r"\udbff\udfff"));
    assert_eq!(
        json_to_lon(&lon_to_json(&supplementary).unwrap()).unwrap(),
        supplementary,
    );
}

#[test]
fn preserves_strings_used_as_object_keys() {
    let value = json!({
        "a b": "space key",
        "line\nkey": "newline key",
        "a:b": "colon key",
        "مرحبا بالعالم": "Arabic key",
        "键 名": "CJK key",
        "a\u{202E}b": "directional key",
    });
    expect_value_and_json_round_trip(&value);
}

#[test]
fn round_trips_grammar_conflict_keys_through_table_schemas() {
    let keys = grammar_conflict_strings();
    let row = |offset: i32| {
        let mut map = serde_json::Map::new();
        for (index, key) in keys.iter().enumerate() {
            map.insert(
                key.as_str().unwrap().to_string(),
                json!(offset + index as i32),
            );
        }
        Value::Object(map)
    };
    let value = json!([row(0), row(keys.len() as i32)]);

    let encoded = LonCodec::new().encode(&value).unwrap();
    assert!(encoded.starts_with('[') && encoded.contains(';'));
    expect_value_and_json_round_trip(&value);
}

#[test]
fn preserves_duplicate_delimiter_heavy_keys_in_the_text_api() {
    let json = r#"{"a;b":1,"a;b":2,"q\"x":3,"line\nkey":4,"":5}"#;
    let encoded = json_to_lon(json).unwrap();
    assert_eq!(lon_to_json(&encoded).unwrap(), json);
    assert_eq!(
        json_to_lon(&lon_to_json(&encoded).unwrap()).unwrap(),
        encoded
    );
}

#[test]
fn preserves_a_long_adversarial_string() {
    let fragment = "abc 12 34 \nxxx\nb \txx 😀 مرحبًا 你好 \\ \" \u{202E}";
    let value = Value::String(fragment.repeat(4096));
    expect_value_and_json_round_trip(&value);
}

#[test]
fn rejects_malformed_quoted_strings() {
    for source in [
        "\"unterminated",
        r#""bad\x20""#,
        r#""bad\u12""#,
        r#""bad\u0xz0""#,
        "\"raw\nnewline\"",
        "\"raw\u{0}control\"",
        "\"trailing\\",
    ] {
        let error = lon_to_json(source).unwrap_err();
        assert_eq!(error.kind(), lon::ErrorKind::Format, "{source:?}");
    }
}

#[test]
fn handles_raw_del_in_quoted_strings() {
    // Raw DEL (U+007F) is legal JSON and canonicalizes to a \u007f escape...
    assert_eq!(
        json_to_lon(r#"{"a":"x\u007Fy"}"#).unwrap(),
        r#"{a:"x\u007fy"}"#
    );
    // ...but a LON decoder must reject it unescaped, like every other
    // visibility-sensitive code point.
    let error = lon_to_json("{a:\"x\u{7F}y\"}").unwrap_err();
    assert_eq!(error.kind(), lon::ErrorKind::Format);
}
