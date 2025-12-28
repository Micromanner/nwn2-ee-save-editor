use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyBytes};
use std::sync::Arc;
use indexmap::IndexMap;

use crate::parsers::gff::{GffParser, GffValue, LazyStruct};
use crate::parsers::gff::types::{LocalizedString, LocalizedSubstring};
use crate::error::IntoPyErr;
use crate::parsers::gff::writer::GffWriter;
use crate::parsers::gff::error::GffError;

const STRUCT_ID_KEY: &str = "__struct_id__";
const FIELD_TYPES_KEY: &str = "__field_types__";

#[pyclass(name = "GffParser", module = "nwn2_rust")]
pub struct PyGffParser {
    inner: Arc<GffParser>,
}

#[pymethods]
impl PyGffParser {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let parser = GffParser::new(path).map_err(gff_error_to_py_err)?;
        Ok(PyGffParser { inner: parser })
    }

    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<Self> {
        let parser = GffParser::from_bytes(data.to_vec()).map_err(gff_error_to_py_err)?;
        Ok(PyGffParser { inner: parser })
    }

    fn get_file_type(&self) -> String {
        self.inner.file_type.clone()
    }

    fn get_file_version(&self) -> String {
        self.inner.file_version.clone()
    }

    fn has_field(&self, path: &str) -> bool {
        self.inner.get_value(path).is_ok()
    }

    fn get_field(&self, py: Python, path: &str) -> PyResult<Option<Py<PyAny>>> {
        match self.inner.get_value(path) {
            Ok(value) => Ok(Some(gff_value_to_py(py, value)?)),
            Err(_) => Ok(None),
        }
    }

    fn _get_label(&self, index: u32) -> PyResult<String> {
        self.inner.get_label(index)
             .map(|cow| cow.into_owned())
             .map_err(gff_error_to_py_err)
    }

    fn to_json(&self) -> PyResult<String> {
        let struct_id = self.inner.get_struct_id(0).unwrap_or(0xFFFFFFFF);
        let root = LazyStruct::new(self.inner.clone(), 0, struct_id);
        serde_json::to_string(&root).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
        let struct_id = self.inner.get_struct_id(0).unwrap_or(0xFFFFFFFF);
        let root = LazyStruct::new(self.inner.clone(), 0, struct_id);
        lazy_struct_to_py_dict(py, &root)
    }
}

#[pyclass(name = "GffWriter", module = "nwn2_rust")]
pub struct PyGffWriter {
    file_type: String,
    file_version: String,
}

#[pymethods]
impl PyGffWriter {
    #[new]
    #[pyo3(signature = (file_type="BIC ", file_version="V3.2"))]
    fn new(file_type: &str, file_version: &str) -> Self {
        PyGffWriter {
            file_type: file_type.to_string(),
            file_version: file_version.to_string(),
        }
    }

    fn write(&self, py: Python, path: &str, root: Py<PyAny>) -> PyResult<()> {
        let mut writer = GffWriter::new(&self.file_type, &self.file_version);
        let (root_map, root_struct_id) = py_dict_to_gff_struct(py, root.bind(py))?;
        let bytes = writer.write_with_struct_id(root_map, root_struct_id).map_err(gff_error_to_py_err)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    fn dump(&self, py: Python, root: Py<PyAny>) -> PyResult<Py<PyBytes>> {
        let mut writer = GffWriter::new(&self.file_type, &self.file_version);
        let (root_map, root_struct_id) = py_dict_to_gff_struct(py, root.bind(py))?;
        let bytes = writer.write_with_struct_id(root_map, root_struct_id).map_err(gff_error_to_py_err)?;
        Ok(PyBytes::new(py, &bytes).into())
    }
}

/// Convert a LazyStruct to a Python dict with __struct_id__ and __field_types__
fn lazy_struct_to_py_dict(py: Python, lazy: &LazyStruct) -> PyResult<Py<PyAny>> {
    let fields = lazy.force_load();
    let dict = PyDict::new(py);
    let field_types = PyDict::new(py);

    // Add struct ID
    dict.set_item(STRUCT_ID_KEY, lazy.struct_id)?;

    // Add fields and collect their types
    for (label, value) in fields {
        let type_id = get_type_id(&value);
        field_types.set_item(&label, type_id)?;
        dict.set_item(&label, gff_value_to_py(py, value)?)?;
    }

    // Add field types
    dict.set_item(FIELD_TYPES_KEY, field_types)?;

    Ok(dict.into())
}

/// Get the GFF type ID for a value
fn get_type_id(value: &GffValue) -> u32 {
    match value {
        GffValue::Byte(_) => 0,
        GffValue::Char(_) => 1,
        GffValue::Word(_) => 2,
        GffValue::Short(_) => 3,
        GffValue::Dword(_) => 4,
        GffValue::Int(_) => 5,
        GffValue::Dword64(_) => 6,
        GffValue::Int64(_) => 7,
        GffValue::Float(_) => 8,
        GffValue::Double(_) => 9,
        GffValue::String(_) => 10,
        GffValue::ResRef(_) => 11,
        GffValue::LocString(_) => 12,
        GffValue::Void(_) => 13,
        GffValue::Struct(_) | GffValue::StructOwned(_) | GffValue::StructRef(_) => 14,
        GffValue::List(_) | GffValue::ListOwned(_) | GffValue::ListRef(_) => 15,
    }
}

/// Convert GffValue to Python object
fn gff_value_to_py(py: Python, value: GffValue) -> PyResult<Py<PyAny>> {
    match value {
        GffValue::Byte(v) => Ok(v.into_pyobject(py)?.unbind().into()),
        GffValue::Char(v) => Ok(v.to_string().into_pyobject(py)?.unbind().into()),
        GffValue::Word(v) => Ok(v.into_pyobject(py)?.unbind().into()),
        GffValue::Short(v) => Ok(v.into_pyobject(py)?.unbind().into()),
        GffValue::Dword(v) => Ok(v.into_pyobject(py)?.unbind().into()),
        GffValue::Int(v) => Ok(v.into_pyobject(py)?.unbind().into()),
        GffValue::Dword64(v) => Ok(v.into_pyobject(py)?.unbind().into()),
        GffValue::Int64(v) => Ok(v.into_pyobject(py)?.unbind().into()),
        GffValue::Float(v) => Ok(v.into_pyobject(py)?.unbind().into()),
        GffValue::Double(v) => Ok(v.into_pyobject(py)?.unbind().into()),
        GffValue::String(v) => Ok(v.into_owned().into_pyobject(py)?.unbind().into()),
        GffValue::ResRef(v) => Ok(v.into_owned().into_pyobject(py)?.unbind().into()),
        GffValue::LocString(ls) => {
            let dict = PyDict::new(py);
            dict.set_item("string_ref", ls.string_ref as u32)?;
            let subs = PyList::empty(py);
            for sub in ls.substrings {
                let sub_dict = PyDict::new(py);
                sub_dict.set_item("string", sub.string.into_owned())?;
                sub_dict.set_item("language", sub.language)?;
                sub_dict.set_item("gender", sub.gender)?;
                subs.append(sub_dict)?;
            }
            dict.set_item("substrings", subs)?;
            Ok(dict.into())
        },
        GffValue::Void(v) => Ok(PyBytes::new(py, &v).into()),
        GffValue::Struct(lazy) => lazy_struct_to_py_dict(py, &lazy),
        GffValue::List(list) => {
            let py_list = PyList::empty(py);
            for item in list {
                py_list.append(lazy_struct_to_py_dict(py, &item)?)?;
            }
            Ok(py_list.into())
        },
        GffValue::StructOwned(map) => {
            let dict = PyDict::new(py);
            let field_types = PyDict::new(py);
            dict.set_item(STRUCT_ID_KEY, 0u32)?;
            for (k, v) in *map {
                let type_id = get_type_id(&v);
                field_types.set_item(&k, type_id)?;
                dict.set_item(&k, gff_value_to_py(py, v)?)?;
            }
            dict.set_item(FIELD_TYPES_KEY, field_types)?;
            Ok(dict.into())
        },
        GffValue::ListOwned(vec) => {
            let py_list = PyList::empty(py);
            for map in vec {
                let dict = PyDict::new(py);
                let field_types = PyDict::new(py);
                dict.set_item(STRUCT_ID_KEY, 0u32)?;
                for (k, v) in map {
                    let type_id = get_type_id(&v);
                    field_types.set_item(&k, type_id)?;
                    dict.set_item(&k, gff_value_to_py(py, v)?)?;
                }
                dict.set_item(FIELD_TYPES_KEY, field_types)?;
                py_list.append(dict)?;
            }
            Ok(py_list.into())
        },
        GffValue::StructRef(_) | GffValue::ListRef(_) => {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Internal GFF reference types should not be exposed"))
        }
    }
}

/// Convert Python dict to GFF struct with metadata extraction
/// Returns (fields, struct_id)
fn py_dict_to_gff_struct(py: Python, obj: &Bound<'_, PyAny>) -> PyResult<(IndexMap<String, GffValue<'static>>, u32)> {
    let dict: Bound<'_, PyDict> = obj.extract()
        .map_err(|_| PyErr::new::<pyo3::exceptions::PyTypeError, _>("Expected dict for struct"))?;

    // Extract metadata
    let struct_id = dict.get_item(STRUCT_ID_KEY)?
        .map(|v| v.extract::<u32>())
        .transpose()?
        .unwrap_or(0xFFFFFFFF);

    // Extract field_types dict if present
    let field_types_dict: Option<Bound<'_, PyDict>> = dict.get_item(FIELD_TYPES_KEY)?
        .and_then(|v| v.extract::<Bound<'_, PyDict>>().ok());

    let mut map = IndexMap::new();

    for (key, value) in dict.iter() {
        let key_str = key.extract::<String>()?;

        // Skip metadata keys
        if key_str == STRUCT_ID_KEY || key_str == FIELD_TYPES_KEY {
            continue;
        }

        // Get type from __field_types__ if available, otherwise infer
        let type_id: Option<u32> = field_types_dict
            .as_ref()
            .and_then(|ft| {
                // get_item returns PyResult<Option<Bound>>
                ft.get_item(&key_str)
                    .ok()
                    .flatten()
                    .and_then(|v| v.extract::<u32>().ok())
            });

        let gff_value = if let Some(tid) = type_id {
            convert_typed_value(py, &value, tid)?
        } else {
            infer_gff_value(py, &value)?
        };

        map.insert(key_str, gff_value);
    }

    Ok((map, struct_id))
}

/// Infer GFF type from Python value (fallback when no type info)
fn infer_gff_value(py: Python, obj: &Bound<'_, PyAny>) -> PyResult<GffValue<'static>> {
    // Check for bool first (before int, since bool is a subtype of int in Python)
    if let Ok(val) = obj.extract::<bool>() {
        return Ok(GffValue::Byte(if val { 1 } else { 0 }));
    }

    // Check for dict (struct or locstring)
    if obj.is_instance_of::<PyDict>() {
        let dict: Bound<'_, PyDict> = obj.extract()?;
        // Check if it's a locstring (has string_ref key)
        if dict.get_item("string_ref")?.is_some() && dict.get_item(STRUCT_ID_KEY)?.is_none() {
            return convert_typed_value(py, obj, 12); // LOCSTRING
        }
        // Regular struct
        let (mut fields, struct_id) = py_dict_to_gff_struct(py, obj)?;
        // Store struct_id in the map as a special field for writer to extract
        fields.insert(STRUCT_ID_KEY.to_string(), GffValue::Dword(struct_id));
        return Ok(GffValue::StructOwned(Box::new(fields)));
    }

    // Check for list
    if obj.is_instance_of::<PyList>() {
        let list: Bound<'_, PyList> = obj.extract()?;
        let mut vec = Vec::new();
        for item in list.iter() {
            let (mut fields, struct_id) = py_dict_to_gff_struct(py, &item)?;
            // Store struct_id in each list item
            fields.insert(STRUCT_ID_KEY.to_string(), GffValue::Dword(struct_id));
            vec.push(fields);
        }
        return Ok(GffValue::ListOwned(vec));
    }

    // Check for bytes
    if obj.is_instance_of::<PyBytes>() {
        let bytes: Bound<'_, PyBytes> = obj.extract()?;
        return Ok(GffValue::Void(std::borrow::Cow::Owned(bytes.as_bytes().to_vec())));
    }

    // Check for string
    if let Ok(val) = obj.extract::<String>() {
        return Ok(GffValue::String(std::borrow::Cow::Owned(val)));
    }

    // Check for int (default to Int32)
    if let Ok(val) = obj.extract::<i32>() {
        return Ok(GffValue::Int(val));
    }

    // Check for float (default to Double)
    if let Ok(val) = obj.extract::<f64>() {
        return Ok(GffValue::Double(val));
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
        "Unsupported Python type for GFF conversion: {}", obj.get_type()
    )))
}

/// Convert Python value to GFF value with explicit type
fn convert_typed_value(py: Python, obj: &Bound<'_, PyAny>, type_id: u32) -> PyResult<GffValue<'static>> {
    match type_id {
        0 => Ok(GffValue::Byte(obj.extract::<u8>()?)),
        1 => Ok(GffValue::Char(obj.extract::<String>()?.chars().next().unwrap_or('\0'))),
        2 => Ok(GffValue::Word(obj.extract::<u16>()?)),
        3 => Ok(GffValue::Short(obj.extract::<i16>()?)),
        4 => Ok(GffValue::Dword(obj.extract::<u32>()?)),
        5 => Ok(GffValue::Int(obj.extract::<i32>()?)),
        6 => Ok(GffValue::Dword64(obj.extract::<u64>()?)),
        7 => Ok(GffValue::Int64(obj.extract::<i64>()?)),
        8 => Ok(GffValue::Float(obj.extract::<f32>()?)),
        9 => Ok(GffValue::Double(obj.extract::<f64>()?)),
        10 => Ok(GffValue::String(std::borrow::Cow::Owned(obj.extract::<String>()?))),
        11 => Ok(GffValue::ResRef(std::borrow::Cow::Owned(obj.extract::<String>()?))),
        12 => {
            // LocalizedString
            let dict: Bound<'_, PyDict> = obj.extract()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyTypeError, _>("Expected dict for LocString"))?;
            let string_ref = dict.get_item("string_ref")?
                .map(|v| v.extract::<i64>())
                .transpose()?
                .unwrap_or(-1) as i32;

            let mut substrings = Vec::new();
            if let Some(subs) = dict.get_item("substrings")? {
                if let Ok(sub_list) = subs.extract::<Bound<'_, PyList>>() {
                    for sub in sub_list.iter() {
                        let sub_dict: Bound<'_, PyDict> = sub.extract()?;
                        substrings.push(LocalizedSubstring {
                            string: std::borrow::Cow::Owned(
                                sub_dict.get_item("string")?.unwrap().extract::<String>()?
                            ),
                            language: sub_dict.get_item("language")?.unwrap().extract::<u32>()?,
                            gender: sub_dict.get_item("gender")?.unwrap().extract::<u32>()?,
                        });
                    }
                }
            }
            Ok(GffValue::LocString(LocalizedString { string_ref, substrings }))
        },
        13 => {
            let bytes: Bound<'_, PyBytes> = obj.extract()?;
            Ok(GffValue::Void(std::borrow::Cow::Owned(bytes.as_bytes().to_vec())))
        },
        14 => {
            // Struct - preserve struct_id in the map for writer to extract
            let (mut fields, struct_id) = py_dict_to_gff_struct(py, obj)?;
            fields.insert(STRUCT_ID_KEY.to_string(), GffValue::Dword(struct_id));
            Ok(GffValue::StructOwned(Box::new(fields)))
        },
        15 => {
            // List - preserve struct_id in each item for writer to extract
            let list: Bound<'_, PyList> = obj.extract()?;
            let mut vec = Vec::new();
            for item in list.iter() {
                let (mut fields, struct_id) = py_dict_to_gff_struct(py, &item)?;
                fields.insert(STRUCT_ID_KEY.to_string(), GffValue::Dword(struct_id));
                vec.push(fields);
            }
            Ok(GffValue::ListOwned(vec))
        },
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Unknown type ID: {}", type_id))),
    }
}

fn gff_error_to_py_err(err: GffError) -> PyErr {
    match err {
        GffError::InvalidHeader(_) |
        GffError::InvalidVersion(_) |
        GffError::InvalidStructIndex(_) |
        GffError::InvalidFieldIndex(_) |
        GffError::InvalidLabelIndex(_) |
        GffError::UnsupportedFieldType(_) |
        GffError::BufferOverflow(_) => {
             PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string())
        },
        GffError::FieldNotFound(_) => {
             PyErr::new::<pyo3::exceptions::PyKeyError, _>(err.to_string())
        },
        _ => err.into_py_err(), // Fallback to generic mapping (RuntimeError usually)
    }
}
