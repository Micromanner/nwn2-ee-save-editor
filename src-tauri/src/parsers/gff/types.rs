use std::borrow::Cow;
use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::error::GffError;
use super::parser::GffParser;
use crate::utils::text_encoding::decode_localized_text;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GffFieldType {
    Byte = 0,
    Char = 1,
    Word = 2,
    Short = 3,
    Dword = 4,
    Int = 5,
    Dword64 = 6,
    Int64 = 7,
    Float = 8,
    Double = 9,
    String = 10,
    ResRef = 11,
    LocString = 12,
    Void = 13,
    Struct = 14,
    List = 15,
}

/// Decoded GFF text. Bytes that were not valid UTF-8 (a legacy code page) are
/// kept so an unedited string is written back unchanged instead of re-encoding
/// a guessed decode; any edit builds a new value, which is written as UTF-8.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GffText {
    text: String,
    legacy_bytes: Option<Box<[u8]>>,
}

impl GffText {
    pub fn decode(bytes: &[u8], nwn_language_id: u32) -> Self {
        match std::str::from_utf8(bytes) {
            Ok(text) => text.into(),
            Err(_) => Self {
                text: decode_localized_text(bytes, nwn_language_id),
                legacy_bytes: Some(bytes.into()),
            },
        }
    }

    /// Replaces the text, keeping the original bytes when it is unchanged.
    pub fn set(&mut self, text: &str) {
        if self.text != text {
            *self = text.into();
        }
    }

    /// The bytes to write: the original legacy bytes, else UTF-8.
    pub fn encoded(&self) -> &[u8] {
        self.legacy_bytes.as_deref().unwrap_or(self.text.as_bytes())
    }
}

impl std::ops::Deref for GffText {
    type Target = str;

    fn deref(&self) -> &str {
        &self.text
    }
}

impl AsRef<str> for GffText {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl std::fmt::Display for GffText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<String> for GffText {
    fn from(text: String) -> Self {
        Self {
            text,
            legacy_bytes: None,
        }
    }
}

impl From<&str> for GffText {
    fn from(text: &str) -> Self {
        text.to_owned().into()
    }
}

impl PartialEq<str> for GffText {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}

impl PartialEq<&str> for GffText {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}

impl Serialize for GffText {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.text)
    }
}

impl<'de> Deserialize<'de> for GffText {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizedSubstring {
    pub string: GffText,
    pub language: u32,
    pub gender: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizedString {
    pub string_ref: i32,
    pub substrings: Vec<LocalizedSubstring>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum GffValue<'a> {
    Byte(u8),
    Char(char),
    Word(u16),
    Short(i16),
    Dword(u32),
    Int(i32),
    Dword64(u64),
    Int64(i64),
    Float(f32),
    Double(f64),
    String(GffText),
    ResRef(GffText),
    LocString(LocalizedString),

    #[serde(with = "serde_bytes")]
    Void(Cow<'a, [u8]>),

    Struct(LazyStruct),
    List(Vec<LazyStruct>),

    StructOwned(Box<IndexMap<String, GffValue<'a>>>),
    ListOwned(Vec<IndexMap<String, GffValue<'a>>>),

    StructRef(u32),
    ListRef(Vec<u32>),
}

use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct LazyStruct {
    pub parser: Arc<GffParser>,
    pub struct_index: u32,
    pub struct_id: u32,
    pub cached_fields: Arc<RwLock<Option<IndexMap<String, GffValue<'static>>>>>,
}

impl Serialize for LazyStruct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::{Error, SerializeMap};
        let fields = self.force_load().map_err(S::Error::custom)?;
        let mut map = serializer.serialize_map(Some(fields.len()))?;
        for (k, v) in fields {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
    }
}

impl LazyStruct {
    pub fn new(parser: Arc<GffParser>, struct_index: u32, struct_id: u32) -> Self {
        Self {
            parser,
            struct_index,
            struct_id,
            cached_fields: Arc::new(RwLock::new(None)),
        }
    }

    pub fn force_load(&self) -> Result<IndexMap<String, GffValue<'static>>, GffError> {
        if let Some(fields) = self.cached() {
            return Ok(fields);
        }
        let fields = self.read_fields()?;
        if let Ok(mut guard) = self.cached_fields.write() {
            *guard = Some(fields.clone());
        }
        Ok(fields)
    }

    /// Like `force_load`, but consumes the handle and skips filling the cache.
    fn into_fields(self) -> Result<IndexMap<String, GffValue<'static>>, GffError> {
        match self.cached() {
            Some(fields) => Ok(fields),
            None => self.read_fields(),
        }
    }

    fn cached(&self) -> Option<IndexMap<String, GffValue<'static>>> {
        self.cached_fields.read().ok()?.clone()
    }

    fn read_fields(&self) -> Result<IndexMap<String, GffValue<'static>>, GffError> {
        let mut owned_fields: IndexMap<String, GffValue<'static>> = self
            .parser
            .read_struct_fields(self.struct_index)?
            .into_iter()
            .map(|(k, v)| (k, v.into_owned()))
            .collect();

        // Inject struct_id as it is metadata required for properly identifying list items (e.g. Equip_ItemList slot)
        owned_fields.insert("__struct_id__".to_string(), GffValue::Dword(self.struct_id));
        Ok(owned_fields)
    }
}

impl GffValue<'_> {
    pub fn into_owned(self) -> GffValue<'static> {
        match self {
            GffValue::Byte(v) => GffValue::Byte(v),
            GffValue::Char(v) => GffValue::Char(v),
            GffValue::Word(v) => GffValue::Word(v),
            GffValue::Short(v) => GffValue::Short(v),
            GffValue::Dword(v) => GffValue::Dword(v),
            GffValue::Int(v) => GffValue::Int(v),
            GffValue::Dword64(v) => GffValue::Dword64(v),
            GffValue::Int64(v) => GffValue::Int64(v),
            GffValue::Float(v) => GffValue::Float(v),
            GffValue::Double(v) => GffValue::Double(v),
            GffValue::String(text) => GffValue::String(text),
            GffValue::ResRef(text) => GffValue::ResRef(text),
            GffValue::LocString(ls) => GffValue::LocString(ls),
            GffValue::Void(cow) => GffValue::Void(Cow::Owned(cow.into_owned())),
            GffValue::Struct(lazy) => GffValue::Struct(lazy),
            GffValue::List(vec) => GffValue::List(vec),
            GffValue::StructOwned(map) => {
                let owned_map = map.into_iter().map(|(k, v)| (k, v.into_owned())).collect();
                GffValue::StructOwned(Box::new(owned_map))
            }
            GffValue::ListOwned(vec) => {
                let owned_vec = vec
                    .into_iter()
                    .map(|map| map.into_iter().map(|(k, v)| (k, v.into_owned())).collect())
                    .collect();
                GffValue::ListOwned(owned_vec)
            }
            GffValue::StructRef(idx) => GffValue::StructRef(idx),
            GffValue::ListRef(vec) => GffValue::ListRef(vec),
        }
    }

    /// Recursively convert all lazy-loaded structures to fully owned data.
    /// This eliminates all Arc<RwLock<>> from nested LazyStruct values,
    /// making the resulting data suitable for direct ownership without locks.
    pub fn force_owned(self) -> Result<GffValue<'static>, GffError> {
        Ok(match self {
            GffValue::Struct(lazy) => {
                GffValue::StructOwned(Box::new(force_owned_map(lazy.into_fields()?)?))
            }
            GffValue::List(lazy_vec) => GffValue::ListOwned(
                lazy_vec
                    .into_iter()
                    .map(|lazy| force_owned_map(lazy.into_fields()?))
                    .collect::<Result<_, _>>()?,
            ),
            GffValue::StructOwned(map) => GffValue::StructOwned(Box::new(force_owned_map(*map)?)),
            GffValue::ListOwned(vec) => GffValue::ListOwned(
                vec.into_iter()
                    .map(force_owned_map)
                    .collect::<Result<_, _>>()?,
            ),
            other => other.into_owned(),
        })
    }
}

/// `force_owned` for every value of a field map.
pub fn force_owned_map(
    map: IndexMap<String, GffValue<'_>>,
) -> Result<IndexMap<String, GffValue<'static>>, GffError> {
    map.into_iter()
        .map(|(k, v)| Ok((k, v.force_owned()?)))
        .collect()
}
