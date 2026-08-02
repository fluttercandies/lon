# Language Object Notation v1

Language Object Notation (LON) is a lossless encoding of the JSON data model. Its compact form is designed to remain recognizable without a format primer: JSON braces, brackets, colons, strings, numbers, and literals keep their ordinary meaning. LON removes commas and unnecessary quotes, then uses a visible header row to factor repeated record keys.

## Design order

1. Preserve the complete JSON value without inference.
2. Make the value understandable from the payload alone.
3. Minimize tokens, including on nested and non-uniform data.
4. Keep the compact payload readable to people.

LON uses no aliases, key dictionaries, numeric references, hidden schemas, or model instructions.

## General values

Whitespace separates object fields, all-bare-string array items, and table cells. A canonical array uses spaces only when every item is a safely bare string. All other arrays use compact JSON commas, keeping mixed-type and container boundaries visually explicit. A decoder accepts either separator, but canonical output never mixes them within one array.

JSON literals and number syntax are unchanged. A string is bare only when it cannot be confused with a literal, number, or structural token. Numeric-looking strings such as `"01"`, `".5"`, `"1e"`, and `"0x10"` remain quoted. JSON-adjacent literals such as `"NaN"`, `"undefined"`, `"None"`, `"yes"`, and `"off"` also remain quoted so a model cannot silently change their type.

Value classification has one exhaustive precedence order: exact `null`, `true`, and `false` tokens are literals; tokens matching JSON number syntax are numbers; every other permitted bare token is a string. LON has no identifier, symbol, enum, date, or other inferred scalar type. An array uses record-table grammar only when its initial segment is a valid schema terminated by an unquoted top-level semicolon. Semicolons cannot occur in bare tokens or separate ordinary array items, so this does not collide with an ordinary array.

Empty strings, strings containing whitespace, structural punctuation, double quotes, backslashes, controls, or invisible separators use JSON quotes and escapes. An ASCII apostrophe may remain bare only inside a token, never at its first or last position; this keeps contractions and names compact without making surrounding apostrophes look like string delimiters. Unicode text and valid surrogate pairs are preserved; C0/C1 controls, line separators, zero-width spaces, word joiners, and byte-order marks are emitted as visible escapes. A LON decoder requires visibility-sensitive characters and lone surrogates to use escapes even inside quoted strings.

The complete JSON string domain is valid in both values and object member names, including empty names and escaped lone surrogate code units. Object and schema-key context determines that a parsed token is a name, so numeric- or literal-looking names cannot change the value type. The same quoting, visibility, and preservation rules apply to ordinary object keys, nested keys, and table-schema field names.

Serialized LON is UTF-8 and is independent of any programming language's native string representation. Text from Latin, CJK, Cyrillic, Greek, Arabic, Hebrew, Indic, Southeast Asian, African, and historic scripts follows the same rules. An encoder performs no NFC, NFD, case, width, or direction normalization: code points and UTF-16 surrogate escapes retain their exact JSON meaning. Directional controls are escaped visibly without altering the logical text.

Objects retain braces and colons:

```lon
{name:Ada age:37 active:true}
```

Arrays retain brackets:

```lon
[Ada Bob Lin]
[[1,2],[3,4]]
```

These are equivalent to:

```json
{"name":"Ada","age":37,"active":true}
["Ada","Bob","Lin"]
[[1,2],[3,4]]
```


### Dynamic arrays

Mixed and non-uniform arrays keep every value's ordinary delimiters and do not infer a shared schema:

```lon
[1,{a:b},{a:[1,{b:c}]}]
```

This is exactly `[1,{"a":"b"},{"a":[1,{"b":"c"}]}]`. Commas make heterogeneous boundaries immediately recognizable while bare keys and safe bare strings retain the token savings.

Empty containers remain `{}` and `[]`. Object member order and duplicate keys are retained by text-to-text conversion.

## Self-describing record tables

A table removes record keys only when at least two array records have exactly the same ordered top-level field names and the recursively flattened schema contains at least two leaf-value fields. Uniform nested objects are recursively factored into the schema. One-value records, keyed objects, and arrays containing a varying or mixed-shape object cell stay in ordinary explicit form; the encoder does not trade immediate type ownership for a marginal saving.

### Array of records

```text
[SCHEMA;ROW;ROW]
```

Example:

```lon
[id name addr{city zip} active;1 Ada London W1 true;2 Bob Paris "75001" false]
```

Equivalent JSON:

```json
[
  {"id":1,"name":"Ada","addr":{"city":"London","zip":"W1"},"active":true},
  {"id":2,"name":"Bob","addr":{"city":"Paris","zip":"75001"},"active":false}
]
```

Spaces separate schema fields and row cells. The first semicolon ends the schema; each later semicolon starts the next row. A valid table contains at least two rows, and its schema contains at least two leaf-value fields and fixes the exact cell count of every row. A decoder therefore rejects ambiguous empty, one-row, or one-value forms, missing cells, extra cells, malformed separators, and truncated nested values without needing row parentheses or another row delimiter.

Nested object shapes stay visible in the schema: `addr{city zip}` means that `city` and `zip` belong to `addr`. Row values follow the schema depth-first, so repeated nested braces and keys are unnecessary. A nested table retains the same brackets and header-row structure:

```lon
{orders:[id customer{id name} items;A1 1 Ada [sku qty;X 2;Y 1];A2 2 Bob []]}
```

An array with varying object-cell shapes remains explicit:

```lon
[{id:1 payload:{a:b}},{id:2 payload:{x:[1,{b:c}]}}]
```

The outer brackets always decode to an array. A schema with at least two leaf-value fields followed by `;` is the local table discriminator; ordinary arrays use whitespace or commas between values and never use semicolons. Keyed objects remain ordinary objects, preserving familiar `key:value` ownership.

There is deliberately no declared row count. This removes metadata that zero-prompt readers may mistake for a value and saves its tokens. It also means that deletion of an entire syntactically complete row cannot be detected by the notation alone; transports that require omission detection must provide a byte length, checksum, or authenticated envelope. Structural truncation and row-width errors remain detectable.

## Canonical encoding rules

A canonical encoder:

- emits ordinary arrays and objects with JSON delimiters;
- uses spaces only for arrays consisting entirely of safely bare strings, otherwise compact commas;
- omits only those string quotes whose removal cannot change type, structure, or likely model interpretation;
- considers `[SCHEMA;ROW;ROW]` only for two or more array records with the same ordered top-level fields and at least two recursively flattened row values, then emits it when there are at least three rows, when the recursively flattened schema contains at least three field names, or when its structure is no larger than the ordinary form;
- recursively factors only uniform nested objects;
- leaves keyed objects, differing record fields, mixed object/non-object cells, and varying object-cell shapes explicit.

The table compactness gate is deterministic and tokenizer-independent. Let `S` be the emitted schema's UTF-8 byte length, `F` its recursive field-name count, `V` its leaf-value count, and `O` one ordinary record's structural UTF-8 byte length after removing leaf encodings. For `N` rows, table structure costs `2 + S + N * V` bytes. Ordinary array structure costs `N + 1 + N * O` bytes after row values common to both forms are cancelled. Two-row schemas use a table only when the table cost is no greater. Larger row or schema counts use the token-oriented thresholds above because repeated key and colon tokens dominate even at the byte break-even.

A decoder rejects malformed numbers, ambiguous or structurally unsafe bare strings and keys, unescaped visibility-sensitive code points, missing separators, invalid escapes, table row-width mismatches, trailing cells, truncated nesting, and trailing document content.

## UTF-8 transport

A serialized LON document is UTF-8 without a byte-order mark. A byte-oriented decoder must reject ill-formed UTF-8 rather than insert replacement characters. Transports and intermediaries must preserve the byte sequence without Unicode normalization, character-set transcoding, or newline rewriting.

Canonical string serialization escapes every C0/C1 control, CR, LF, Unicode line separator, visibility-sensitive code point, and lone surrogate. Consequently, string data cannot inject a physical line break or LON delimiter into the document. A CLI or transport may append framing whitespace after the complete document; that whitespace is not part of the encoded JSON value.

LON defines a document, not message framing, integrity, or embedding-context escaping. A stream must provide a text-frame boundary, byte length, or equivalent envelope; omission-sensitive use additionally needs a checksum or authenticated envelope. Raw LON must not be inserted into an HTTP header, URL, HTML, JavaScript, shell command, or SQL statement without that context's own escaping or encoding.

## Resource safety

Implementations must bound nesting, parsed or expanded element occurrences, schema width, input size, and materialized output size before the corresponding allocation or multiplication. Element counts alone are not an output bound because a table repeats each schema key into every decoded row. Text codecs report a limit breach as `FormatException`; Dart value encoding uses `ArgumentError` for an over-budget value graph. Neither path returns a partial successful document.

The Dart codec defaults to a nesting depth of 512, 1,000,000 expanded elements, 1,000,000 table schema fields, 67,108,864 input UTF-16 code units, and 67,108,864 output UTF-16 code units. The element limit also applies while encoding Dart object graphs and JSON text, so shared acyclic containers cannot expand without bound. The CLI independently caps streamed UTF-8 input at 67,108,864 bytes and never includes untrusted source text in diagnostics.

## Round-trip contract

For valid JSON text `J`, `lonToJson(jsonToLon(J))` produces compact JSON with the same JSON value, member order, duplicate members, member-name and string-value code units, and number lexemes. JSON whitespace and equivalent string-escape spellings are canonicalized.

For canonical LON `L`, `jsonToLon(lonToJson(L)) == L`.
