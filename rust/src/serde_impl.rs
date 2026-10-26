//! Serde integration: serialize any `Serialize` value directly to canonical
//! LON and deserialize any `DeserializeOwned` type from it.
//!
//! Both directions bridge through the shared [`Node`] tree, so canonical
//! output, record-table factoring, parsing limits, and diagnostics are
//! identical to the text-to-text and value codecs.
//!
//! Semantics, aligned with the [`crate::LonCodec`] value API:
//!
//! - Integers keep their exact value; floats use Rust's shortest round-trip
//!   `Display` form. Non-finite floats and `i128`/`u128` values outside the
//!   `i64`/`u64` ranges fail serialization.
//! - Structs and maps emit members in serializer visitation order, so output
//!   determinism follows the container: `BTreeMap` sorts, `HashMap` does not.
//! - On decoding, duplicate object keys keep their first position and the
//!   last value wins. Strings holding escaped lone surrogates decode with
//!   U+FFFD, matching `serde_json`.
//! - Enums follow the `serde_json` conventions: unit variants serialize as
//!   the bare variant-name string, other variants as `{"Variant": ...}`
//!   (external tagging). Internally and adjacently tagged enums use the same
//!   object representation through the map path.
//! - `deserialize_any` is fully supported, so `#[serde(untagged)]` enums and
//!   `serde_json::Value` targets work.

use crate::error::LonError;
use crate::json_string::{string_from_units_lossy, units_from_str};
use crate::node::{Member, Node};
use serde::de::{self, DeserializeOwned, DeserializeSeed, EnumAccess, Visitor};
use serde::ser::{self, Serialize};

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// Serializes `value` into a node tree, enforcing the codec's depth and
/// element limits exactly like the `serde_json::Value` path.
pub(crate) fn to_node(
    codec: &crate::LonCodec,
    value: &(impl Serialize + ?Sized),
) -> Result<Node, LonError> {
    let mut budget = Budget {
        max_depth: codec.max_depth,
        max_elements: codec.max_expanded_elements,
        elements: 0,
    };
    value.serialize(Ser {
        budget: &mut budget,
        depth: 0,
    })
}

/// Deserializes a type from an already parsed node tree.
pub(crate) fn from_node<T: DeserializeOwned>(node: &Node) -> Result<T, LonError> {
    T::deserialize(NodeDeserializer(node))
}

/// Decodes UTF-16 code units with lone surrogates replaced by U+FFFD, the
/// same policy as the `serde_json::Value` codec.
pub(crate) fn string_from_units(units: &[u16]) -> String {
    string_from_units_lossy(units)
}

// ---------------------------------------------------------------------------
// Serializer: Serialize -> Node
// ---------------------------------------------------------------------------

/// Shared depth and element budget across the whole value tree.
struct Budget {
    max_depth: usize,
    max_elements: usize,
    elements: usize,
}

impl Budget {
    fn enter(&self, depth: usize) -> Result<(), LonError> {
        if depth > self.max_depth {
            return Err(LonError::invalid_value(format!(
                "Maximum nesting depth of {} exceeded",
                self.max_depth
            )));
        }
        Ok(())
    }

    fn reserve(&mut self) -> Result<(), LonError> {
        if self.elements >= self.max_elements {
            return Err(LonError::invalid_value(format!(
                "Expanded element limit of {} exceeded",
                self.max_elements
            )));
        }
        self.elements += 1;
        Ok(())
    }
}

/// The serializer for one value: a mutable borrow of the shared budget plus
/// the container depth that value sits at.
struct Ser<'a> {
    budget: &'a mut Budget,
    depth: usize,
}

impl Ser<'_> {
    /// Serializes a container child: reserves one element and descends.
    fn child(&mut self, value: &(impl Serialize + ?Sized)) -> Result<Node, LonError> {
        let depth = self.depth + 1;
        self.budget.enter(depth)?;
        self.budget.reserve()?;
        value.serialize(Ser {
            budget: self.budget,
            depth,
        })
    }
}

fn invalid(message: impl Into<String>) -> LonError {
    LonError::invalid_value(message)
}

impl<'a> serde::Serializer for Ser<'a> {
    type Ok = Node;
    type Error = LonError;
    type SerializeSeq = SeqState<'a>;
    type SerializeTuple = SeqState<'a>;
    type SerializeTupleStruct = SeqState<'a>;
    type SerializeTupleVariant = TupleVariantState<'a>;
    type SerializeMap = MapState<'a>;
    type SerializeStruct = MapState<'a>;
    type SerializeStructVariant = StructVariantState<'a>;

    fn serialize_bool(self, v: bool) -> Result<Node, LonError> {
        Ok(Node::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Node, LonError> {
        self.serialize_i64(i64::from(v))
    }
    fn serialize_i16(self, v: i16) -> Result<Node, LonError> {
        self.serialize_i64(i64::from(v))
    }
    fn serialize_i32(self, v: i32) -> Result<Node, LonError> {
        self.serialize_i64(i64::from(v))
    }
    fn serialize_i64(self, v: i64) -> Result<Node, LonError> {
        Ok(Node::Number(v.to_string()))
    }
    fn serialize_i128(self, v: i128) -> Result<Node, LonError> {
        i64::try_from(v)
            .map(|v| Node::Number(v.to_string()))
            .map_err(|_| invalid("i128 value out of range for the JSON data model"))
    }

    fn serialize_u8(self, v: u8) -> Result<Node, LonError> {
        self.serialize_u64(u64::from(v))
    }
    fn serialize_u16(self, v: u16) -> Result<Node, LonError> {
        self.serialize_u64(u64::from(v))
    }
    fn serialize_u32(self, v: u32) -> Result<Node, LonError> {
        self.serialize_u64(u64::from(v))
    }
    fn serialize_u64(self, v: u64) -> Result<Node, LonError> {
        Ok(Node::Number(v.to_string()))
    }
    fn serialize_u128(self, v: u128) -> Result<Node, LonError> {
        u64::try_from(v)
            .map(|v| Node::Number(v.to_string()))
            .map_err(|_| invalid("u128 value out of range for the JSON data model"))
    }

    fn serialize_f32(self, v: f32) -> Result<Node, LonError> {
        if !v.is_finite() {
            return Err(invalid("JSON numbers must be finite"));
        }
        Ok(Node::Number(v.to_string()))
    }
    fn serialize_f64(self, v: f64) -> Result<Node, LonError> {
        if !v.is_finite() {
            return Err(invalid("JSON numbers must be finite"));
        }
        Ok(Node::Number(v.to_string()))
    }

    fn serialize_char(self, v: char) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_str(self, v: &str) -> Result<Node, LonError> {
        Ok(Node::String(units_from_str(v)))
    }

    fn serialize_bytes(mut self, v: &[u8]) -> Result<Node, LonError> {
        // Serde's data model has no byte-string kind; like serde_json, bytes
        // become an array of numbers.
        let mut values = Vec::with_capacity(v.len());
        for byte in v {
            let node = self.child(&u64::from(*byte))?;
            values.push(node);
        }
        Ok(Node::Array(values))
    }

    fn serialize_none(self) -> Result<Node, LonError> {
        Ok(Node::Null)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Node, LonError> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Node, LonError> {
        Ok(Node::Null)
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Node, LonError> {
        Ok(Node::Null)
    }
    /// Externally tagged unit variant: the bare variant-name string.
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Node, LonError> {
        Ok(Node::String(units_from_str(variant)))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Node, LonError> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Node, LonError> {
        Ok(variant_wrapper(variant, value.serialize(self)?))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, LonError> {
        Ok(SeqState {
            ser: self,
            values: Vec::with_capacity(len.unwrap_or(0)),
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, LonError> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, LonError> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, LonError> {
        Ok(TupleVariantState {
            variant,
            ser: self,
            values: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, LonError> {
        Ok(MapState {
            ser: self,
            members: Vec::new(),
            key: None,
        })
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, LonError> {
        Ok(MapState {
            ser: self,
            members: Vec::with_capacity(len),
            key: None,
        })
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, LonError> {
        Ok(StructVariantState {
            variant,
            ser: self,
            members: Vec::with_capacity(len),
        })
    }
}

fn variant_wrapper(variant: &str, value: Node) -> Node {
    Node::Object(vec![Member {
        key: units_from_str(variant),
        value,
    }])
}

struct SeqState<'a> {
    ser: Ser<'a>,
    values: Vec<Node>,
}

impl ser::SerializeSeq for SeqState<'_> {
    type Ok = Node;
    type Error = LonError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), LonError> {
        self.ser.child(value).map(|node| self.values.push(node))
    }
    fn end(self) -> Result<Node, LonError> {
        Ok(Node::Array(self.values))
    }
}

impl ser::SerializeTuple for SeqState<'_> {
    type Ok = Node;
    type Error = LonError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), LonError> {
        ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Node, LonError> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SeqState<'_> {
    type Ok = Node;
    type Error = LonError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), LonError> {
        ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Node, LonError> {
        ser::SerializeSeq::end(self)
    }
}

struct TupleVariantState<'a> {
    variant: &'static str,
    ser: Ser<'a>,
    values: Vec<Node>,
}

impl ser::SerializeTupleVariant for TupleVariantState<'_> {
    type Ok = Node;
    type Error = LonError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), LonError> {
        self.ser.child(value).map(|node| self.values.push(node))
    }
    fn end(self) -> Result<Node, LonError> {
        Ok(variant_wrapper(self.variant, Node::Array(self.values)))
    }
}

struct MapState<'a> {
    ser: Ser<'a>,
    members: Vec<Member>,
    key: Option<Vec<u16>>,
}

impl MapState<'_> {
    fn take_key(&mut self) -> Result<Vec<u16>, LonError> {
        self.key
            .take()
            .ok_or_else(|| invalid("serialize_value called before serialize_key"))
    }
}

impl ser::SerializeMap for MapState<'_> {
    type Ok = Node;
    type Error = LonError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), LonError> {
        if self.key.is_some() {
            return Err(invalid(
                "serialize_key called twice without serialize_value",
            ));
        }
        let node = key.serialize(KeySerializer)?;
        let Node::String(units) = node else {
            return Err(invalid(
                "LON object keys must be strings; convert the key to a String first",
            ));
        };
        self.key = Some(units);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), LonError> {
        let key = self.take_key()?;
        let value = self.ser.child(value)?;
        self.members.push(Member { key, value });
        Ok(())
    }

    fn end(mut self) -> Result<Node, LonError> {
        if self.key.is_some() {
            return Err(invalid("serialize_value missing for the last map key"));
        }
        self.members.shrink_to_fit();
        Ok(Node::Object(self.members))
    }
}

impl ser::SerializeStruct for MapState<'_> {
    type Ok = Node;
    type Error = LonError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), LonError> {
        ser::SerializeMap::serialize_key(self, key)?;
        ser::SerializeMap::serialize_value(self, value)
    }
    fn end(self) -> Result<Node, LonError> {
        ser::SerializeMap::end(self)
    }
}

struct StructVariantState<'a> {
    variant: &'static str,
    ser: Ser<'a>,
    members: Vec<Member>,
}

impl ser::SerializeStructVariant for StructVariantState<'_> {
    type Ok = Node;
    type Error = LonError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), LonError> {
        // One element per field: `child` does the reserving, exactly like the
        // parser and the `serde_json::Value` converter count one member.
        let value = self.ser.child(value)?;
        self.members.push(Member {
            key: units_from_str(key),
            value,
        });
        Ok(())
    }
    fn end(self) -> Result<Node, LonError> {
        Ok(variant_wrapper(self.variant, Node::Object(self.members)))
    }
}

/// Serializes a map key. Only string-producing primitives are accepted,
/// matching serde_json's JSON object-key rules.
struct KeySerializer;

impl serde::Serializer for KeySerializer {
    type Ok = Node;
    type Error = LonError;
    type SerializeSeq = ser::Impossible<Node, LonError>;
    type SerializeTuple = ser::Impossible<Node, LonError>;
    type SerializeTupleStruct = ser::Impossible<Node, LonError>;
    type SerializeTupleVariant = ser::Impossible<Node, LonError>;
    type SerializeMap = ser::Impossible<Node, LonError>;
    type SerializeStruct = ser::Impossible<Node, LonError>;
    type SerializeStructVariant = ser::Impossible<Node, LonError>;

    fn serialize_bool(self, v: bool) -> Result<Node, LonError> {
        self.serialize_str(if v { "true" } else { "false" })
    }
    fn serialize_i8(self, v: i8) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i16(self, v: i16) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i32(self, v: i32) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i64(self, v: i64) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_i128(self, v: i128) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u8(self, v: u8) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u16(self, v: u16) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u32(self, v: u32) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u64(self, v: u64) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_u128(self, v: u128) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_f32(self, v: f32) -> Result<Node, LonError> {
        if !v.is_finite() {
            return Err(invalid("JSON map keys must be finite numbers"));
        }
        self.serialize_str(&v.to_string())
    }
    fn serialize_f64(self, v: f64) -> Result<Node, LonError> {
        if !v.is_finite() {
            return Err(invalid("JSON map keys must be finite numbers"));
        }
        self.serialize_str(&v.to_string())
    }
    fn serialize_char(self, v: char) -> Result<Node, LonError> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_str(self, v: &str) -> Result<Node, LonError> {
        Ok(Node::String(units_from_str(v)))
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Node, LonError> {
        Err(invalid("byte strings cannot be LON object keys"))
    }
    fn serialize_none(self) -> Result<Node, LonError> {
        Err(invalid("None cannot be a LON object key"))
    }
    fn serialize_some<T: ?Sized + Serialize>(self, _value: &T) -> Result<Node, LonError> {
        Err(invalid("Option cannot be a LON object key"))
    }
    fn serialize_unit(self) -> Result<Node, LonError> {
        Err(invalid("unit cannot be a LON object key"))
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Node, LonError> {
        Err(invalid("unit structs cannot be LON object keys"))
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Node, LonError> {
        self.serialize_str(variant)
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Node, LonError> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Node, LonError> {
        Err(invalid("enum variants cannot be LON object keys"))
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, LonError> {
        Err(invalid("sequences cannot be LON object keys"))
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, LonError> {
        Err(invalid("sequences cannot be LON object keys"))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, LonError> {
        Err(invalid("sequences cannot be LON object keys"))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, LonError> {
        Err(invalid("sequences cannot be LON object keys"))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, LonError> {
        Err(invalid("maps cannot be LON object keys"))
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, LonError> {
        Err(invalid("maps cannot be LON object keys"))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, LonError> {
        Err(invalid("maps cannot be LON object keys"))
    }
}

// ---------------------------------------------------------------------------
// Deserializer: Node -> Deserialize
// ---------------------------------------------------------------------------

struct NodeDeserializer<'a>(&'a Node);

impl<'de> de::Deserializer<'de> for NodeDeserializer<'_> {
    type Error = LonError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        match self.0 {
            Node::Null => visitor.visit_unit(),
            Node::Bool(boolean) => visitor.visit_bool(*boolean),
            Node::Number(lexeme) => {
                if !lexeme.contains('.') && !lexeme.contains(['e', 'E']) {
                    if let Ok(integer) = lexeme.parse::<i64>() {
                        return visitor.visit_i64(integer);
                    }
                    if let Ok(unsigned) = lexeme.parse::<u64>() {
                        return visitor.visit_u64(unsigned);
                    }
                }
                match lexeme.parse::<f64>() {
                    Ok(number) if number.is_finite() => visitor.visit_f64(number),
                    _ => Err(LonError::format("Number out of range")),
                }
            }
            Node::String(units) => visitor.visit_str(&string_from_units(units)),
            Node::Array(values) => visitor.visit_seq(SeqDeserializer {
                iter: values.iter(),
            }),
            Node::Object(members) => visitor.visit_map(MapDeserializer {
                pending: None,
                iter: members.iter(),
            }),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        match self.0 {
            Node::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, LonError> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        match self.0 {
            Node::Number(lexeme) => match lexeme.parse::<i128>() {
                Ok(number) => visitor.visit_i128(number),
                // No lexeme echo: the token may be arbitrarily long source text.
                Err(_) => Err(de::Error::invalid_value(
                    de::Unexpected::Other("number out of range"),
                    &"an integer number",
                )),
            },
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        match self.0 {
            Node::Number(lexeme) => match lexeme.parse::<u128>() {
                Ok(number) => visitor.visit_u128(number),
                Err(_) => Err(de::Error::invalid_value(
                    de::Unexpected::Other("number out of range"),
                    &"an unsigned integer number",
                )),
            },
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, LonError> {
        match self.0 {
            // Externally tagged unit variant: the bare variant-name string.
            Node::String(units) => visitor.visit_enum(UnitVariantEnumAccess {
                name: &string_from_units(units),
            }),
            // Any other externally tagged variant: a single-member object.
            Node::Object(members) if members.len() == 1 => {
                visitor.visit_enum(VariantDeserializer {
                    name: &string_from_units(&members[0].key),
                    value: &members[0].value,
                })
            }
            node => Err(de::Error::invalid_type(
                de::Unexpected::Other(if matches!(node, Node::Object(_)) {
                    "object with multiple members"
                } else {
                    "non-string value"
                }),
                &"a string or single-member object for an enum",
            )),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
        bytes byte_buf seq tuple tuple_struct map struct identifier unit
        unit_struct ignored_any
    }
}

struct StrDeserializer<'a>(&'a str);

impl<'de> de::Deserializer<'de> for StrDeserializer<'_> {
    type Error = LonError;
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf seq tuple tuple_struct map struct identifier unit
        unit_struct ignored_any newtype_struct enum option
    }
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        visitor.visit_str(self.0)
    }
}

struct SeqDeserializer<'a> {
    iter: std::slice::Iter<'a, Node>,
}

impl<'de> de::SeqAccess<'de> for SeqDeserializer<'_> {
    type Error = LonError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, LonError> {
        match self.iter.next() {
            Some(node) => seed.deserialize(NodeDeserializer(node)).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

struct MapDeserializer<'a> {
    pending: Option<&'a Node>,
    iter: std::slice::Iter<'a, Member>,
}

impl<'de> de::MapAccess<'de> for MapDeserializer<'_> {
    type Error = LonError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, LonError> {
        match self.iter.next() {
            Some(Member { key, value }) => {
                let result = seed.deserialize(MapKeyDeserializer(&string_from_units(key)));
                self.pending = Some(value);
                result.map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, LonError> {
        let value = self
            .pending
            .take()
            .ok_or_else(|| de::Error::custom("map value requested before key"))?;
        seed.deserialize(NodeDeserializer(value))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len() + usize::from(self.pending.is_some()))
    }
}

/// Deserializes object keys: strings for struct fields and string-keyed maps,
/// parsed numbers and booleans for integer- or bool-keyed maps, mirroring
/// serde_json's key handling.
struct MapKeyDeserializer<'a>(&'a str);

impl<'de> de::Deserializer<'de> for MapKeyDeserializer<'_> {
    type Error = LonError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        visitor.visit_str(self.0)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        match self.0 {
            "true" => visitor.visit_bool(true),
            "false" => visitor.visit_bool(false),
            _ => Err(de::Error::invalid_value(
                de::Unexpected::Str(self.0),
                &"a boolean object key",
            )),
        }
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        match self.0.parse::<i128>() {
            Ok(number) => visitor.visit_i128(number),
            Err(_) => Err(self.invalid_number()),
        }
    }

    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        match self.0.parse::<u128>() {
            Ok(number) => visitor.visit_u128(number),
            Err(_) => Err(self.invalid_number()),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, LonError> {
        visitor.visit_enum(UnitVariantEnumAccess { name: self.0 })
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        visitor.visit_some(self)
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        self.visit_number(visitor)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        self.visit_number(visitor)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        self.visit_number(visitor)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        self.visit_number(visitor)
    }
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        self.visit_number(visitor)
    }
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        self.visit_number(visitor)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        self.visit_number(visitor)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        self.visit_number(visitor)
    }
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        self.visit_number(visitor)
    }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, LonError> {
        self.visit_number(visitor)
    }

    serde::forward_to_deserialize_any! {
        char str string identifier ignored_any bytes byte_buf seq tuple
        tuple_struct map struct unit unit_struct newtype_struct
    }
}

impl MapKeyDeserializer<'_> {
    /// Number keys keep serde_json's behavior: the string lexeme is parsed
    /// and handed to the visitor, which range-checks it.
    fn visit_number<'de, V: Visitor<'de>>(&self, visitor: V) -> Result<V::Value, LonError> {
        if !self.0.contains('.') && !self.0.contains(['e', 'E']) {
            if let Ok(integer) = self.0.parse::<i64>() {
                return visitor.visit_i64(integer);
            }
            if let Ok(unsigned) = self.0.parse::<u64>() {
                return visitor.visit_u64(unsigned);
            }
        }
        match self.0.parse::<f64>() {
            Ok(number) if number.is_finite() => visitor.visit_f64(number),
            _ => Err(self.invalid_number()),
        }
    }

    fn invalid_number(&self) -> LonError {
        de::Error::invalid_value(de::Unexpected::Str(self.0), &"a numeric object key")
    }
}

struct VariantDeserializer<'a> {
    name: &'a str,
    value: &'a Node,
}

/// Enum access for a bare variant-name string: the variant carries no
/// payload, so only `unit_variant` succeeds.
struct UnitVariantEnumAccess<'a> {
    name: &'a str,
}

impl<'de> EnumAccess<'de> for UnitVariantEnumAccess<'_> {
    type Error = LonError;
    type Variant = UnitVariantPayload;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), LonError> {
        let name = seed.deserialize(StrDeserializer(self.name))?;
        Ok((name, UnitVariantPayload))
    }
}

struct UnitVariantPayload;

impl<'de> de::VariantAccess<'de> for UnitVariantPayload {
    type Error = LonError;

    fn unit_variant(self) -> Result<(), LonError> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, _seed: T) -> Result<T::Value, LonError> {
        Err(de::Error::invalid_type(
            de::Unexpected::UnitVariant,
            &"a newtype variant with a payload",
        ))
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, LonError> {
        Err(de::Error::invalid_type(
            de::Unexpected::UnitVariant,
            &"a tuple variant with a payload",
        ))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, LonError> {
        Err(de::Error::invalid_type(
            de::Unexpected::UnitVariant,
            &"a struct variant with fields",
        ))
    }
}

impl<'de, 'a> EnumAccess<'de> for VariantDeserializer<'a> {
    type Error = LonError;
    type Variant = VariantValueDeserializer<'a>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), LonError> {
        let name = seed.deserialize(StrDeserializer(self.name))?;
        Ok((name, VariantValueDeserializer { value: self.value }))
    }
}

struct VariantValueDeserializer<'a> {
    value: &'a Node,
}

impl<'de> de::VariantAccess<'de> for VariantValueDeserializer<'_> {
    type Error = LonError;

    fn unit_variant(self) -> Result<(), LonError> {
        match self.value {
            Node::Null => Ok(()),
            _ => Err(de::Error::invalid_type(
                de::Unexpected::Other("non-null value"),
                &"the unit variant's null",
            )),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, LonError> {
        seed.deserialize(NodeDeserializer(self.value))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, LonError> {
        match self.value {
            Node::Array(values) => visitor.visit_seq(SeqDeserializer {
                iter: values.iter(),
            }),
            _ => Err(de::Error::invalid_type(
                de::Unexpected::Other("non-array value"),
                &"a tuple variant's array",
            )),
        }
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, LonError> {
        match self.value {
            Node::Object(members) => visitor.visit_map(MapDeserializer {
                pending: None,
                iter: members.iter(),
            }),
            _ => Err(de::Error::invalid_type(
                de::Unexpected::Other("non-object value"),
                &"a struct variant's object",
            )),
        }
    }
}
