use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyModule};

use super::error::{SecurityLimits, TDAError};
use super::parser::load_multiple_files;
use super::types::TDAParser as RustTDAParser;

/// Python wrapper for the Rust TDAParser with full API compatibility
#[pyclass(name = "TDAParser", module = "rust_tda_parser")]
pub struct PyTDAParser {
    inner: RustTDAParser,
}

#[pymethods]
impl PyTDAParser {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustTDAParser::new(),
        }
    }

    /// Parse 2DA data from bytes
    fn parse_from_bytes(&mut self, data: &Bound<'_, PyBytes>) -> PyResult<()> {
        self.inner
            .parse_from_bytes(data.as_bytes())
            .map_err(convert_tda_error)
    }

    /// Read and parse a 2DA file from disk
    fn read(&mut self, file_path: &str) -> PyResult<()> {
        self.inner
            .parse_from_file(file_path)
            .map_err(convert_tda_error)
    }

    /// Load from file with optional caching
    fn load_with_cache(&mut self, source_path: &str, cache_path: Option<&str>) -> PyResult<bool> {
        if let Some(cache_path) = cache_path {
            self.inner
                .load_with_cache(source_path, Some(cache_path))
                .map_err(convert_tda_error)
        } else {
            self.inner
                .load_with_cache(source_path, None::<&str>)
                .map_err(convert_tda_error)
        }
    }

    /// Get the number of data rows
    fn get_resource_count(&self) -> usize {
        self.inner.row_count()
    }

    /// Get the number of columns
    fn get_column_count(&self) -> usize {
        self.inner.column_count()
    }
    
    /// Alias for get_column_count for compatibility
    fn column_count(&self) -> usize {
        self.inner.column_count()
    }

    /// Get list of column labels
    fn get_column_labels(&self) -> Vec<String> {
        self.inner.column_names().into_iter().map(|s| s.to_string()).collect()
    }
    
    /// Alias for get_resource_count for compatibility
    fn row_count(&self) -> usize {
        self.inner.row_count()
    }

    /// Get a string value by row index and column (compatible with original API)
    fn get_string(&self, resource_index: usize, column: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
        let result = if let Ok(col_name) = column.extract::<String>() {
            self.inner.get_cell_by_name(resource_index, &col_name)
        } else if let Ok(col_index) = column.extract::<usize>() {
            self.inner.get_cell(resource_index, col_index)
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Column must be string or integer"
            ));
        };

        result
            .map(|opt| opt.map(|s| s.to_string()))
            .map_err(convert_tda_error)
    }

    /// Get an integer value by row index and column
    fn get_int(&self, resource_index: usize, column: &Bound<'_, PyAny>) -> PyResult<Option<i64>> {
        let string_value = self.get_string(resource_index, column)?;
        
        match string_value {
            Some(value) => {
                if value.is_empty() {
                    Ok(None)
                } else {
                    // Try parsing as integer, then as float->integer
                    match value.parse::<i64>() {
                        Ok(int_val) => Ok(Some(int_val)),
                        Err(_) => match value.parse::<f64>() {
                            Ok(float_val) => Ok(Some(float_val as i64)),
                            Err(_) => Ok(None),
                        }
                    }
                }
            }
            None => Ok(None),
        }
    }

    /// Get a float value by row index and column
    fn get_float(&self, resource_index: usize, column: &Bound<'_, PyAny>) -> PyResult<Option<f64>> {
        let string_value = self.get_string(resource_index, column)?;
        
        match string_value {
            Some(value) => {
                if value.is_empty() {
                    Ok(None)
                } else {
                    match value.parse::<f64>() {
                        Ok(float_val) => Ok(Some(float_val)),
                        Err(_) => Ok(None),
                    }
                }
            }
            None => Ok(None),
        }
    }

    /// Get a boolean value by row index and column
    fn get_bool(&self, resource_index: usize, column: &Bound<'_, PyAny>) -> PyResult<Option<bool>> {
        let string_value = self.get_string(resource_index, column)?;
        
        match string_value {
            Some(value) => {
                if value.is_empty() {
                    Ok(None)
                } else {
                    let bool_val = match value.as_str() {
                        "1" | "true" | "True" | "TRUE" | "yes" | "Yes" | "YES" => Some(true),
                        "0" | "false" | "False" | "FALSE" | "no" | "No" | "NO" => Some(false),
                        _ => None,
                    };
                    Ok(bool_val)
                }
            }
            None => Ok(None), // **** values return None consistently across all accessor methods
        }
    }

    /// Find first row where column equals value
    fn find_row(&self, column: &str, value: &str) -> PyResult<Option<usize>> {
        self.inner
            .find_row(column, value)
            .map_err(convert_tda_error)
    }

    /// Get entire row as dictionary (main compatibility method)
    fn get_row_dict(&self, resource_index: usize, py: Python) -> PyResult<Option<Py<PyAny>>> {
        let row_data = self.inner
            .get_row_dict(resource_index)
            .map_err(convert_tda_error)?;

        let py_dict = PyDict::new(py);
        for (key, value) in row_data {
            let py_value = match value {
                Some(s) => s.into_pyobject(py)?.into_any().unbind(),
                None => py.None(),
            };
            py_dict.set_item(key, py_value)?;
        }

        Ok(Some(py_dict.into()))
    }

    /// Get all rows as a list of dictionaries (batch operation for performance)
    /// This is much faster than calling get_row_dict() in a loop due to single
    /// Python/Rust boundary crossing instead of N crossings.
    fn get_all_rows_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
        use pyo3::types::PyList;

        let all_rows = self.inner.get_all_rows_dict();

        let py_list: Vec<Py<PyAny>> = all_rows.into_iter().map(|row_data| {
            let py_dict = PyDict::new(py);
            for (key, value) in row_data {
                let py_value = match value {
                    Some(s) => s.into_pyobject(py).unwrap().into_any().unbind(),
                    None => py.None(),
                };
                py_dict.set_item(key, py_value).unwrap();
            }
            py_dict.into()
        }).collect();

        Ok(PyList::new(py, py_list)?.into())
    }

    /// Set security limits
    fn set_max_file_size(&mut self, size: usize) {
        // Update the existing parser's security limits without losing data
        self.inner.security_limits_mut().max_file_size = size;
    }

    /// Serialize to compressed MessagePack
    fn to_msgpack_bytes(&self, py: Python) -> PyResult<Py<PyAny>> {
        let data = self.inner
            .to_msgpack_compressed()
            .map_err(convert_tda_error)?;
        Ok(PyBytes::new(py, &data).into())
    }

    /// Deserialize from compressed MessagePack
    #[staticmethod]
    fn from_msgpack_bytes(data: &Bound<'_, PyBytes>) -> PyResult<Self> {
        let parser = RustTDAParser::from_msgpack_compressed(data.as_bytes())
            .map_err(convert_tda_error)?;
        Ok(Self { inner: parser })
    }

    /// Get parser statistics
    fn get_statistics(&self, py: Python) -> PyResult<Py<PyAny>> {
        let stats = self.inner.statistics();
        let dict = PyDict::new(py);

        dict.set_item("total_cells", stats.total_cells)?;
        dict.set_item("memory_usage", stats.memory_usage)?;
        dict.set_item("interned_strings", stats.interned_strings)?;
        dict.set_item("parse_time_ms", stats.parse_time_ms)?;
        dict.set_item("compression_ratio", stats.compression_ratio)?;

        Ok(dict.into())
    }

    /// Get parser metadata
    fn get_metadata(&self, py: Python) -> PyResult<Py<PyAny>> {
        let metadata = self.inner.metadata();
        let dict = PyDict::new(py);

        dict.set_item("file_size", metadata.file_size)?;
        dict.set_item("line_count", metadata.line_count)?;
        dict.set_item("parse_time_ns", metadata.parse_time_ns)?;
        dict.set_item("has_warnings", metadata.has_warnings)?;
        dict.set_item("format_version", &metadata.format_version)?;

        Ok(dict.into())
    }

    /// Clear all data
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Python representation
    fn __repr__(&self) -> String {
        format!(
            "TDAParser(columns={}, rows={}, memory={}KB)",
            self.inner.column_count(),
            self.inner.row_count(),
            self.inner.memory_usage() / 1024
        )
    }

    /// Python string representation
    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Python property compatibility - expose columns attribute
    #[getter]
    fn columns(&self) -> Vec<String> {
        self.get_column_labels()
    }

    /// Python property compatibility - allow setting columns (for tests)
    #[setter]
    fn set_columns(&mut self, columns: Vec<String>) {
        use super::types::ColumnInfo;
        
        // Clear existing columns
        self.inner.columns_mut().clear();
        self.inner.column_map_mut().clear();
        
        // Add new columns
        for (index, column_name) in columns.iter().enumerate() {
            let symbol = self.inner.interner_mut().get_or_intern(column_name);
            let column_info = ColumnInfo {
                name: symbol,
                index,
            };
            self.inner.columns_mut().push(column_info);
            self.inner.column_map_mut().insert(column_name.to_lowercase(), index);
        }
    }

    /// Python property compatibility - expose resources attribute
    #[getter]
    fn resources(&self) -> Vec<Vec<String>> {
        let mut resources = Vec::new();
        for row_idx in 0..self.inner.row_count() {
            let mut row = Vec::new();
            for col_idx in 0..self.inner.column_count() {
                match self.inner.get_cell(row_idx, col_idx) {
                    Ok(Some(value)) => row.push(value.to_string()),
                    Ok(None) => row.push("****".to_string()),
                    Err(_) => row.push("".to_string()),
                }
            }
            resources.push(row);
        }
        resources
    }

    /// Python property compatibility - allow setting resources (for tests)
    #[setter]
    fn set_resources(&mut self, resources: Vec<Vec<String>>) {
        use super::types::{TDARow, CellValue};
        
        // Clear existing rows
        self.inner.rows_mut().clear();
        
        // Add new rows
        for resource_row in resources {
            let mut row = TDARow::new();
            for cell_data in resource_row {
                let cell_value = if cell_data == "****" {
                    CellValue::Empty
                } else {
                    CellValue::new(&cell_data, self.inner.interner_mut())
                };
                row.push(cell_value);
            }
            self.inner.rows_mut().push(row);
        }
    }

    /// Python property compatibility - expose column_map attribute
    #[getter]
    fn column_map(&self, py: Python) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        let column_names = self.inner.column_names();
        for (index, name) in column_names.iter().enumerate() {
            dict.set_item(name.to_lowercase(), index)?;
        }
        Ok(dict.into())
    }

    /// Python property compatibility - allow setting column_map (for tests)
    #[setter]
    fn set_column_map(&mut self, column_map: std::collections::HashMap<String, usize>) {
        // Clear existing column map
        self.inner.column_map_mut().clear();
        
        // Add new mappings
        for (name, index) in column_map {
            self.inner.column_map_mut().insert(name.to_lowercase(), index);
        }
    }
}

/// Top-level function for parallel loading of multiple files
#[pyfunction]
fn load_all_2da_files(
    paths: Vec<String>,
    max_file_size: Option<usize>,
    py: Python,
) -> PyResult<Py<PyAny>> {
    let limits = max_file_size.map(|size| SecurityLimits {
        max_file_size: size,
        ..SecurityLimits::default()
    });

    let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
    let parsers = load_multiple_files(&path_refs, limits).map_err(convert_tda_error)?;

    let py_dict = PyDict::new(py);
    for (path, parser) in parsers {
        let py_parser = Py::new(py, PyTDAParser { inner: parser })?;
        py_dict.set_item(path, py_parser)?;
    }

    Ok(py_dict.into())
}

/// Convert Rust TDAError to Python exception
fn convert_tda_error(error: TDAError) -> PyErr {
    match error {
        TDAError::InvalidHeader(_) | TDAError::MalformedLine { .. } | TDAError::ParseError { .. } => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(error.to_string())
        }
        TDAError::FileSizeExceeded { .. } | TDAError::SecurityViolation { .. } => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
        }
        TDAError::ColumnNotFound { .. } | TDAError::RowIndexOutOfBounds { .. } | TDAError::ColumnIndexOutOfBounds { .. } => {
            PyErr::new::<pyo3::exceptions::PyIndexError, _>(error.to_string())
        }
        TDAError::InvalidUtf8 { .. } => {
            PyErr::new::<pyo3::exceptions::PyUnicodeDecodeError, _>(error.to_string())
        }
        TDAError::IoError(_) => {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(error.to_string())
        }
        _ => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
        }
    }
}

/// Python module definition
pub fn rust_tda_parser(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTDAParser>()?;
    m.add_function(wrap_pyfunction!(load_all_2da_files, m)?)?;

    // Add alias for backwards compatibility
    m.add("TDAParser", _py.get_type::<PyTDAParser>())?;

    // Add version info
    m.add("__version__", "0.1.0")?;
    m.add("__author__", "NWN2 Enhanced Edition Editor Team")?;

    Ok(())
}