use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Default)]
pub struct XmlData {
    pub integers: HashMap<String, i32>,
    pub strings: HashMap<String, String>,
    pub floats: HashMap<String, f32>,
    pub vectors: HashMap<String, Vector3>,
}

// Intermediate structs for XML Serialization/Deserialization

#[derive(Debug, Deserialize, Serialize)]
pub struct IntegerEntry {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Value")]
    pub value: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IntegersWrapper {
    #[serde(rename = "Integer", default)]
    pub entries: Vec<IntegerEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StringEntry {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Value")]
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StringsWrapper {
    #[serde(rename = "String", default)]
    pub entries: Vec<StringEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FloatEntry {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Value")]
    pub value: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FloatsWrapper {
    #[serde(rename = "Float", default)]
    pub entries: Vec<FloatEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VectorEntry {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "X")]
    pub x: f32,
    #[serde(rename = "Y")]
    pub y: f32,
    #[serde(rename = "Z")]
    pub z: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VectorsWrapper {
    #[serde(rename = "Vector", default)]
    pub entries: Vec<VectorEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "Globals")]
pub struct GlobalsXml {
    #[serde(rename = "Integers", default)]
    pub integers: Option<IntegersWrapper>,
    #[serde(rename = "Strings", default)]
    pub strings: Option<StringsWrapper>,
    #[serde(rename = "Floats", default)]
    pub floats: Option<FloatsWrapper>,
    #[serde(rename = "Vectors", default)]
    pub vectors: Option<VectorsWrapper>,
}

impl XmlData {
    pub fn from_xml_struct(xml: GlobalsXml) -> Self {
        let mut data = XmlData::default();

        if let Some(wrapper) = xml.integers {
            for entry in wrapper.entries {
                data.integers.insert(entry.name, entry.value);
            }
        }
        if let Some(wrapper) = xml.strings {
            for entry in wrapper.entries {
                data.strings.insert(entry.name, entry.value);
            }
        }
        if let Some(wrapper) = xml.floats {
            for entry in wrapper.entries {
                data.floats.insert(entry.name, entry.value);
            }
        }
        if let Some(wrapper) = xml.vectors {
            for entry in wrapper.entries {
                data.vectors.insert(entry.name, Vector3 { x: entry.x, y: entry.y, z: entry.z });
            }
        }
        data
    }

    pub fn to_xml_struct(&self) -> GlobalsXml {
        let mut integers = Vec::new();
        for (k, v) in &self.integers {
            integers.push(IntegerEntry { name: k.clone(), value: *v });
        }
        // Sort for consistent output? The python one doesn't seem to strictly sort, 
        // but it iterates over dict items. Python > 3.7 maintains insertion order.
        // HashMaps don't. Maybe we should sort by name to be nice.
        integers.sort_by(|a, b| a.name.cmp(&b.name));

        let mut strings = Vec::new();
        for (k, v) in &self.strings {
            strings.push(StringEntry { name: k.clone(), value: v.clone() });
        }
        strings.sort_by(|a, b| a.name.cmp(&b.name));

        let mut floats = Vec::new();
        for (k, v) in &self.floats {
            floats.push(FloatEntry { name: k.clone(), value: *v });
        }
        floats.sort_by(|a, b| a.name.cmp(&b.name));

        let mut vectors = Vec::new();
        for (k, v) in &self.vectors {
            vectors.push(VectorEntry { name: k.clone(), x: v.x, y: v.y, z: v.z });
        }
        vectors.sort_by(|a, b| a.name.cmp(&b.name));

        GlobalsXml {
            integers: Some(IntegersWrapper { entries: integers }),
            strings: Some(StringsWrapper { entries: strings }),
            floats: Some(FloatsWrapper { entries: floats }),
            vectors: Some(VectorsWrapper { entries: vectors }),
        }
    }
}
