//! Throughput benchmarks for the codec's five public entry paths over three
//! deterministically generated payloads (small ~100 B, medium ~10 KB, large
//! ~1 MB). Text-input scenarios report throughput against input bytes; the
//! serde `to_string` scenario reports against its serialized LON output.
//!
//! Payload content is fixed by a splitmix64 stream seeded from the row index,
//! so every run measures identical inputs and no files are read.

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde::{Deserialize, Serialize};

/// Deterministic 64-bit mixer (splitmix64 finalizer).
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

const WORDS: [&str; 31] = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet",
    "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra", "tango",
    "uniform", "victor", "whiskey", "xray", "yankee", "zulu", "record", "signal", "matrix", "简体",
    "portal",
];

fn word(hash: u64) -> &'static str {
    WORDS[(hash % WORDS.len() as u64) as usize]
}

fn sentence(seed: u64, words: usize) -> String {
    let mut text = String::with_capacity(words * 7);
    for i in 0..words {
        if i > 0 {
            text.push(' ');
        }
        text.push_str(word(mix(seed.wrapping_add(i as u64))));
    }
    text
}

/// The serde benchmark record: six fields of mixed kinds.
#[derive(Serialize, Deserialize)]
struct Row {
    id: u32,
    name: String,
    score: f64,
    active: bool,
    city: String,
    dept: String,
}

fn row(index: u64) -> Row {
    let h = mix(index);
    Row {
        id: (index % 100_000) as u32,
        name: format!("user_{}", mix(h) % 100_000),
        score: f64::from((h % 10_000) as u32) / 100.0,
        active: h & 1 == 0,
        city: word(mix(h)).to_owned(),
        dept: word(mix(h ^ 0xABCD)).to_owned(),
    }
}

fn row_json(index: u64) -> String {
    let r = row(index);
    format!(
        r#"{{"id":{},"name":"{}","score":{},"active":{},"city":"{}","dept":"{}"}}"#,
        r.id, r.name, r.score, r.active, r.city, r.dept,
    )
}

/// One benchmark payload: its JSON and LON text forms plus the serde rows.
struct Payload {
    name: &'static str,
    json: String,
    lon: String,
    rows: Vec<Row>,
    lon_rows_len: usize,
}

fn payload_small() -> Payload {
    let json = r#"{"id":7,"name":"Ada Lovelace","tags":["math","pioneer"],"score":98.5,"active":true,"note":null}"#.to_owned();
    let rows = vec![row(1), row(2)];
    Payload {
        name: "small",
        lon: lon::json_to_lon(&json).unwrap(),
        lon_rows_len: lon::to_string(&rows).unwrap().len(),
        json,
        rows,
    }
}

fn payload_medium() -> Payload {
    let rows: Vec<Row> = (0..100).map(row).collect();
    let mut json = String::with_capacity(12 * 1024);
    json.push('[');
    for i in 0..rows.len() {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&row_json(i as u64));
    }
    json.push(']');
    Payload {
        name: "medium",
        lon: lon::json_to_lon(&json).unwrap(),
        lon_rows_len: lon::to_string(&rows).unwrap().len(),
        json,
        rows,
    }
}

/// ~1 MB mix of uniform record tables, nested objects, and string-dense text.
fn payload_large() -> Payload {
    let mut json = String::with_capacity(1_100_000);
    json.push_str(r#"{"records":["#);
    for i in 0..3000u64 {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&row_json(i));
    }
    json.push_str(r#"],"nested":["#);
    for i in 0..260u64 {
        let h = mix(i ^ 0x5EED);
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            r#"{{"id":{},"meta":{{"created":"2026-10-{:02}T{:02}:00:00Z","flags":["{}","{}","{}"],"owner":{{"name":"{} {}","role":"{}"}},"weight":{}}}}}"#,
            i,
            1 + i % 28,
            i % 24,
            word(mix(h)),
            word(mix(h + 1)),
            word(mix(h + 2)),
            word(mix(h + 3)),
            word(mix(h + 4)),
            word(mix(h + 5)),
            (h % 1000) as f64 / 8.0,
        ));
    }
    json.push_str(r#"],"messages":["#);
    for i in 0..460u64 {
        if i > 0 {
            json.push(',');
        }
        let text = sentence(i ^ 0xBEEF, 230);
        json.push('"');
        json.push_str(&text);
        json.push('"');
    }
    json.push_str("]}");

    let rows: Vec<Row> = (0..8000).map(row).collect();
    Payload {
        name: "large",
        lon: lon::json_to_lon(&json).unwrap(),
        lon_rows_len: lon::to_string(&rows).unwrap().len(),
        json,
        rows,
    }
}

fn payloads() -> Vec<Payload> {
    vec![payload_small(), payload_medium(), payload_large()]
}

fn bench_codec(c: &mut Criterion) {
    let codec = lon::LonCodec::new();
    let all = payloads();
    if std::env::var_os("LON_BENCH_SIZES").is_some() {
        for payload in &all {
            println!(
                "{}: json {} B, lon {} B, rows {} -> LON {} B",
                payload.name,
                payload.json.len(),
                payload.lon.len(),
                payload.rows.len(),
                payload.lon_rows_len,
            );
        }
    }

    let mut group = c.benchmark_group("codec");
    group
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3));

    for payload in &all {
        let json = &payload.json;
        let lon = &payload.lon;
        group.throughput(Throughput::Bytes(json.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("json_to_lon", payload.name),
            json,
            |b, json| b.iter(|| codec.json_to_lon(black_box(json)).unwrap()),
        );

        group.throughput(Throughput::Bytes(lon.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("lon_to_json", payload.name),
            lon,
            |b, lon| b.iter(|| codec.lon_to_json(black_box(lon)).unwrap()),
        );
        group.bench_with_input(
            BenchmarkId::new("decode_value", payload.name),
            lon,
            |b, lon| b.iter(|| codec.decode(black_box(lon)).unwrap()),
        );
        group.bench_with_input(
            BenchmarkId::new("from_str_value", payload.name),
            lon,
            |b, lon| b.iter(|| lon::from_str::<serde_json::Value>(black_box(lon)).unwrap()),
        );

        group.throughput(Throughput::Bytes(payload.lon_rows_len as u64));
        group.bench_with_input(
            BenchmarkId::new("serde_to_string", payload.name),
            &payload.rows,
            |b, rows| b.iter(|| codec.to_string(black_box(rows)).unwrap()),
        );
    }
    group.finish();
}

criterion_group!(benches, bench_codec);
criterion_main!(benches);
