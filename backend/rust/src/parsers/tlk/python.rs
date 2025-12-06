use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyModule};

use super::error::{SecurityLimits, TLKError};
use super::parser::load_multiple_files;
use super::types::{TLKParser as RustTLKParser, SearchOptions};

/// Python wrapper for the Rust TLKParser with full API compatibility
#[pyclass(name = "TLKParser", module = "rust_tlk_parser")]
pub struct PyTLKParser {
    inner: RustTLKParser,
}

#[pymethods]
impl PyTLKParser {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustTLKParser::new(),
        }
    }

    /// Read and parse a TLK file from disk (main compatibility method)
    fn read(&mut self, file_path: &str) -> PyResult<()> {
        self.inner
            .parse_from_file(file_path)
            .map_err(convert_tlk_error)
    }

    /// Parse TLK data from bytes
    fn parse_from_bytes(&mut self, data: &Bound<'_, PyBytes>) -> PyResult<()> {
        self.inner
            .parse_from_bytes(data.as_bytes())
            .map_err(convert_tlk_error)
    }

    /// Load from file with optional caching
    fn load_with_cache(&mut self, source_path: &str, cache_path: Option<&str>) -> PyResult<bool> {
        if let Some(cache_path) = cache_path {
            self.inner
                .load_with_cache(source_path, Some(cache_path))
                .map_err(convert_tlk_error)
        } else {
            self.inner
                .load_with_cache(source_path, None::<&str>)
                .map_err(convert_tlk_error)
        }
    }

    /// Get a string by reference ID (main compatibility method)
    fn get_string(&mut self, str_ref: i32) -> PyResult<Option<String>> {
        if str_ref < 0 {
            return Ok(None);
        }
        
        self.inner
            .get_string(str_ref as usize)
            .map_err(convert_tlk_error)
    }

    /// Get multiple strings at once (new performance method)
    fn get_strings_batch(&mut self, str_refs: Vec<i32>, py: Python) -> PyResult<Py<PyAny>> {
        let valid_refs: Vec<usize> = str_refs.into_iter()
            .filter_map(|r| if r >= 0 { Some(r as usize) } else { None })
            .collect();

        let batch_result = self.inner
            .get_strings_batch(&valid_refs)
            .map_err(convert_tlk_error)?;

        let py_dict = PyDict::new(py);
        for (str_ref, string) in batch_result.strings {
            py_dict.set_item(str_ref as i32, string)?;
        }

        Ok(py_dict.into())
    }

    /// Get all strings in a range (compatibility method)
    fn get_all_strings(&mut self, start: i32, count: i32, py: Python) -> PyResult<Py<PyAny>> {
        if start < 0 || count < 0 {
            return Ok(PyDict::new(py).into());
        }

        let strings = self.inner
            .get_all_strings(start as usize, count as usize)
            .map_err(convert_tlk_error)?;

        let py_dict = PyDict::new(py);
        for (str_ref, string) in strings {
            py_dict.set_item(str_ref as i32, string)?;
        }

        Ok(py_dict.into())
    }

    /// Search for strings containing the given text
    fn search_strings(&mut self, search_text: &str, case_sensitive: Option<bool>) -> PyResult<Vec<i32>> {
        let options = SearchOptions {
            case_sensitive: case_sensitive.unwrap_or(false),
            ..Default::default()
        };
        
        let results = self.inner
            .search_strings(search_text, &options)
            .map_err(convert_tlk_error)?;
        
        Ok(results.into_iter().map(|r| r.str_ref as i32).collect())
    }

    /// Find first string containing the given text
    fn find_string(&mut self, search_text: &str) -> PyResult<Option<i32>> {
        let result = self.inner
            .find_string(search_text)
            .map_err(convert_tlk_error)?;
        
        Ok(result.map(|r| r as i32))
    }

    /// Get file information (compatibility method)
    fn get_info(&self, py: Python) -> PyResult<Py<PyAny>> {
        let info = self.inner.get_info();
        let py_dict = PyDict::new(py);

        for (key, value) in info {
            match value {
                serde_json::Value::String(s) => py_dict.set_item(key, s)?,
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        py_dict.set_item(key, i)?;
                    } else if let Some(f) = n.as_f64() {
                        py_dict.set_item(key, f)?;
                    }
                }
                serde_json::Value::Bool(b) => py_dict.set_item(key, b)?,
                serde_json::Value::Null => py_dict.set_item(key, py.None())?,
                _ => py_dict.set_item(key, value.to_string())?,
            }
        }

        Ok(py_dict.into())
    }

    /// Get the number of strings
    fn string_count(&self) -> usize {
        self.inner.string_count()
    }

    /// Check if parser has loaded data
    fn is_loaded(&self) -> bool {
        self.inner.is_loaded()
    }

    /// Get memory usage in bytes
    fn memory_usage(&self) -> usize {
        self.inner.memory_usage()
    }

    /// Set security limits
    fn set_max_file_size(&mut self, size: usize) {
        self.inner.security_limits_mut().max_file_size = size;
    }

    fn set_max_strings(&mut self, count: usize) {
        self.inner.security_limits_mut().max_strings = count;
    }

    fn set_max_string_size(&mut self, size: usize) {
        self.inner.security_limits_mut().max_string_size = size;
    }

    /// Serialize to compressed MessagePack
    fn to_msgpack_bytes(&self, py: Python) -> PyResult<Py<PyAny>> {
        let serializable = self.inner.to_serializable();
        let encoded = rmp_serde::to_vec(&serializable)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        // Compress
        use flate2::{Compression, write::GzEncoder};
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&encoded)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let compressed = encoder.finish()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyBytes::new(py, &compressed).into())
    }

    /// Deserialize from compressed MessagePack
    #[staticmethod]
    fn from_msgpack_bytes(data: &Bound<'_, PyBytes>) -> PyResult<Self> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        // Decompress
        let mut decoder = GzDecoder::new(data.as_bytes());
        let mut encoded = Vec::new();
        decoder.read_to_end(&mut encoded)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let serializable: super::types::SerializableTLKParser = rmp_serde::from_slice(&encoded)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let parser = RustTLKParser::from_serializable(serializable);
        Ok(Self { inner: parser })
    }

    /// Get parser statistics
    fn get_statistics(&self, py: Python) -> PyResult<Py<PyAny>> {
        let stats = self.inner.statistics();
        let dict = PyDict::new(py);

        dict.set_item("total_strings", stats.total_strings)?;
        dict.set_item("memory_usage", stats.memory_usage)?;
        dict.set_item("interned_strings", stats.interned_strings)?;
        dict.set_item("parse_time_ms", stats.parse_time_ms)?;
        dict.set_item("cache_hit_ratio", stats.cache_hit_ratio)?;
        dict.set_item("compression_ratio", stats.compression_ratio)?;
        dict.set_item("corrupted_entries", stats.corrupted_entries)?;

        Ok(dict.into())
    }

    /// Get file metadata
    fn get_metadata(&self, py: Python) -> PyResult<Py<PyAny>> {
        let metadata = self.inner.metadata();
        let dict = PyDict::new(py);

        dict.set_item("file_size", metadata.file_size)?;
        dict.set_item("parse_time_ns", metadata.parse_time_ns)?;
        dict.set_item("has_warnings", metadata.has_warnings)?;
        dict.set_item("format_version", &metadata.format_version)?;
        dict.set_item("language_id", metadata.language_id)?;

        if let Some(ref path) = metadata.file_path {
            dict.set_item("file_path", path)?;
        } else {
            dict.set_item("file_path", py.None())?;
        }

        Ok(dict.into())
    }

    /// Clear all data
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Python representation
    fn __repr__(&self) -> String {
        format!(
            "TLKParser(strings={}, memory={}KB, loaded={})",
            self.inner.string_count(),
            self.inner.memory_usage() / 1024,
            self.inner.is_loaded()
        )
    }

    /// Python string representation
    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Python length
    fn __len__(&self) -> usize {
        self.inner.string_count()
    }

    /// Python getitem support for easy access
    fn __getitem__(&mut self, str_ref: i32) -> PyResult<Option<String>> {
        self.get_string(str_ref)
    }

    /// Python contains support
    fn __contains__(&self, str_ref: i32) -> bool {
        if str_ref < 0 {
            return false;
        }
        (str_ref as usize) < self.inner.string_count()
    }

    // Additional compatibility methods for drop-in replacement

    /// Compatibility with original Python API
    #[getter]
    fn file_path(&self) -> Option<String> {
        self.inner.metadata().file_path.clone()
    }

    /// Compatibility with original Python API
    #[getter]
    fn header(&self, py: Python) -> PyResult<Py<PyAny>> {
        if let Some(ref header) = self.inner.header {
            let dict = PyDict::new(py);
            dict.set_item("file_type", &header.file_type)?;
            dict.set_item("version", &header.version)?;
            dict.set_item("language_id", header.language_id)?;
            dict.set_item("string_count", header.string_count)?;
            dict.set_item("string_data_offset", header.string_data_offset)?;
            Ok(dict.into())
        } else {
            Ok(py.None())
        }
    }

    /// Compatibility with original Python API
    #[getter]
    fn string_entries(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);

        for entry in &self.inner.entries {
            let dict = PyDict::new(py);
            dict.set_item("flags", entry.flags)?;
            dict.set_item("sound_resref", &entry.sound_resref)?;
            dict.set_item("volume_variance", entry.volume_variance)?;
            dict.set_item("pitch_variance", entry.pitch_variance)?;
            dict.set_item("offset", entry.data_offset)?;
            dict.set_item("length", entry.string_size)?;
            dict.set_item("present", entry.is_present())?;
            list.append(dict)?;
        }

        Ok(list.into())
    }
}

/// Top-level function for parallel loading of multiple files
#[pyfunction]
fn load_all_tlk_files(
    paths: Vec<String>,
    max_file_size: Option<usize>,
    py: Python,
) -> PyResult<Py<PyAny>> {
    let limits = max_file_size.map(|size| SecurityLimits {
        max_file_size: size,
        ..SecurityLimits::default()
    });

    let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
    let parsers = load_multiple_files(&path_refs, limits).map_err(convert_tlk_error)?;

    let py_dict = PyDict::new(py);
    for (path, parser) in parsers {
        let py_parser = Py::new(py, PyTLKParser { inner: parser })?;
        py_dict.set_item(path, py_parser)?;
    }

    Ok(py_dict.into())
}

/// Convert Rust TLKError to Python exception
fn convert_tlk_error(error: TLKError) -> PyErr {
    match error {
        TLKError::InvalidHeader { .. } | TLKError::FileTooShort { .. } => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(error.to_string())
        }
        TLKError::CorruptedStringEntry { .. } | TLKError::SecurityViolation { .. } => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
        }
        TLKError::StringRefOutOfBounds { .. } => {
            PyErr::new::<pyo3::exceptions::PyIndexError, _>(error.to_string())
        }
        TLKError::InvalidUtf8 { .. } => {
            PyErr::new::<pyo3::exceptions::PyUnicodeDecodeError, _>(error.to_string())
        }
        TLKError::IoError(_) => {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(error.to_string())
        }
        TLKError::FileSizeExceeded { .. } => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
        }
        _ => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
        }
    }
}

/// Python module definition
pub fn rust_tlk_parser(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTLKParser>()?;
    m.add_function(wrap_pyfunction!(load_all_tlk_files, m)?)?;

    // Add alias for backwards compatibility
    m.add("TLKParser", _py.get_type::<PyTLKParser>())?;

    // Add version info
    m.add("__version__", "0.1.0")?;
    m.add("__author__", "NWN2 Enhanced Edition Editor Team")?;

    Ok(())
}