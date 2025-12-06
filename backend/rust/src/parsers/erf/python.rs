use super::error::ErfError;
use super::parser::ErfParser;
use super::types::extension_to_resource_type;
use pyo3::exceptions::{PyIOError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use std::path::Path;

#[pyclass(name = "ErfParser", module = "rust_erf_parser")]
pub struct PyErfParser {
    inner: ErfParser,
}

#[pymethods]
impl PyErfParser {
    #[new]
    fn new() -> Self {
        Self {
            inner: ErfParser::new(),
        }
    }
    
    #[pyo3(signature = (file_path))]
    fn read(&mut self, file_path: &str) -> PyResult<()> {
        self.inner.read(file_path).map_err(erf_to_py_err)
    }
    
    #[pyo3(signature = (data))]
    fn parse_from_bytes(&mut self, data: &Bound<'_, PyBytes>) -> PyResult<()> {
        let bytes = data.as_bytes();
        self.inner.parse_from_bytes(bytes).map_err(erf_to_py_err)
    }
    
    #[pyo3(signature = (resource_type=None))]
    fn list_resources(&self, py: Python, resource_type: Option<u16>) -> PyResult<Py<PyList>> {
        let resources = self.inner.list_resources(resource_type);
        let list = PyList::empty(py);
        
        for (name, size, res_type) in resources {
            let dict = PyDict::new(py);
            dict.set_item("name", name)?;
            dict.set_item("size", size)?;
            dict.set_item("type", res_type)?;
            list.append(dict)?;
        }
        
        Ok(list.into())
    }
    
    #[pyo3(signature = (resource_name, output_path=None))]
    fn extract_resource(&mut self, py: Python, resource_name: &str, output_path: Option<&str>) -> PyResult<Py<PyBytes>> {
        let data = self.inner.extract_resource(resource_name).map_err(erf_to_py_err)?;
        
        if let Some(path) = output_path {
            std::fs::write(path, &data).map_err(|e| PyIOError::new_err(e.to_string()))?;
        }
        
        Ok(PyBytes::new(py, &data).into())
    }
    
    #[pyo3(signature = (output_dir))]
    fn extract_all_2da(&mut self, output_dir: &str) -> PyResult<Vec<String>> {
        self.inner.extract_all_2da(Path::new(output_dir)).map_err(erf_to_py_err)
    }
    
    #[pyo3(signature = (resource_type, output_dir))]
    fn extract_all_by_type(&mut self, resource_type: u16, output_dir: &str) -> PyResult<Vec<String>> {
        self.inner.extract_all_by_type(resource_type, Path::new(output_dir)).map_err(erf_to_py_err)
    }
    
    #[pyo3(signature = (extension, output_dir))]
    fn extract_all_by_extension(&mut self, extension: &str, output_dir: &str) -> PyResult<Vec<String>> {
        let resource_type = extension_to_resource_type(extension)
            .ok_or_else(|| PyValueError::new_err(format!("Unknown extension: {}", extension)))?;
        
        self.inner.extract_all_by_type(resource_type, Path::new(output_dir)).map_err(erf_to_py_err)
    }
    
    fn get_module_info(&mut self, py: Python) -> PyResult<Option<Py<PyBytes>>> {
        match self.inner.get_module_info().map_err(erf_to_py_err)? {
            Some(data) => Ok(Some(PyBytes::new(py, &data).into())),
            None => Ok(None),
        }
    }
    
    fn get_header(&self, py: Python) -> PyResult<Option<Py<PyDict>>> {
        if let Some(header) = &self.inner.header {
            let dict = PyDict::new(py);
            dict.set_item("file_type", &header.file_type)?;
            dict.set_item("version", &header.version)?;
            dict.set_item("language_count", header.language_count)?;
            dict.set_item("localized_string_size", header.localized_string_size)?;
            dict.set_item("entry_count", header.entry_count)?;
            dict.set_item("offset_to_localized_string", header.offset_to_localized_string)?;
            dict.set_item("offset_to_key_list", header.offset_to_key_list)?;
            dict.set_item("offset_to_resource_list", header.offset_to_resource_list)?;
            dict.set_item("build_year", header.build_year)?;
            dict.set_item("build_day", header.build_day)?;
            dict.set_item("description_str_ref", header.description_str_ref)?;
            Ok(Some(dict.into()))
        } else {
            Ok(None)
        }
    }
    
    fn get_statistics(&self, py: Python) -> PyResult<Py<PyDict>> {
        let stats = self.inner.get_statistics();
        let dict = PyDict::new(py);
        
        dict.set_item("total_resources", stats.total_resources)?;
        dict.set_item("total_size", stats.total_size)?;
        dict.set_item("parse_time_ms", stats.parse_time_ms)?;
        
        let types_dict = PyDict::new(py);
        for (res_type, count) in &stats.resource_types {
            types_dict.set_item(res_type.to_string(), count)?;
        }
        dict.set_item("resource_types", types_dict)?;
        
        if let Some((name, size)) = &stats.largest_resource {
            let largest = PyDict::new(py);
            largest.set_item("name", name)?;
            largest.set_item("size", size)?;
            dict.set_item("largest_resource", largest)?;
        }
        
        Ok(dict.into())
    }
    
    fn get_metadata(&self, py: Python) -> PyResult<Option<Py<PyDict>>> {
        if let Some(metadata) = &self.inner.metadata {
            let dict = PyDict::new(py);
            dict.set_item("file_path", &metadata.file_path)?;
            dict.set_item("file_size", metadata.file_size)?;
            dict.set_item("erf_type", &metadata.erf_type)?;
            dict.set_item("version", &metadata.version)?;
            dict.set_item("build_date", &metadata.build_date)?;
            Ok(Some(dict.into()))
        } else {
            Ok(None)
        }
    }
    
    fn to_msgpack_bytes(&self, py: Python) -> PyResult<Py<PyBytes>> {
        let header = self.inner.header.as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("No ERF loaded"))?;
        
        let data = rmp_serde::to_vec(header)
            .map_err(|e| PyRuntimeError::new_err(format!("Serialization failed: {}", e)))?;
        
        Ok(PyBytes::new(py, &data).into())
    }
    
    fn clear_cache(&mut self) {
        self.inner.clear_cache();
    }
    
    #[pyo3(signature = (max_file_size=None, max_resource_count=None, max_resource_size=None))]
    fn set_security_limits(
        &mut self,
        max_file_size: Option<usize>,
        max_resource_count: Option<usize>,
        max_resource_size: Option<usize>,
    ) {
        if let Some(size) = max_file_size {
            self.inner.security_limits.max_file_size = size;
        }
        if let Some(count) = max_resource_count {
            self.inner.security_limits.max_resource_count = count;
        }
        if let Some(size) = max_resource_size {
            self.inner.security_limits.max_resource_size = size;
        }
    }
    
    fn get_resource_count(&self) -> usize {
        self.inner.resources.len()
    }
    
    fn get_erf_type(&self) -> Option<String> {
        self.inner.erf_type.map(|t| t.as_str().to_string())
    }
    
    fn get_version(&self) -> Option<String> {
        self.inner.version.map(|v| match v {
            super::types::ErfVersion::V10 => "V1.0",
            super::types::ErfVersion::V11 => "V1.1",
        }.to_string())
    }
    
    fn has_resource(&self, name: &str) -> bool {
        self.inner.resources.contains_key(&name.to_lowercase())
    }
    
    fn get_resource_size(&self, name: &str) -> Option<u32> {
        self.inner.resources
            .get(&name.to_lowercase())
            .map(|res| res.entry.size)
    }
    
    fn get_resource_type(&self, name: &str) -> Option<u16> {
        self.inner.resources
            .get(&name.to_lowercase())
            .map(|res| res.key.resource_type)
    }
    
    fn __repr__(&self) -> String {
        format!(
            "ErfParser(type={:?}, version={:?}, resources={})",
            self.inner.erf_type.map(|t| t.as_str()).unwrap_or("None"),
            self.inner.version.map(|v| match v {
                super::types::ErfVersion::V10 => "V1.0",
                super::types::ErfVersion::V11 => "V1.1",
            }).unwrap_or("None"),
            self.inner.resources.len()
        )
    }
    
    fn __len__(&self) -> usize {
        self.inner.resources.len()
    }
    
    fn __contains__(&self, name: &str) -> bool {
        self.has_resource(name)
    }
}

fn erf_to_py_err(err: ErfError) -> PyErr {
    match err {
        ErfError::IoError(e) => PyIOError::new_err(e.to_string()),
        ErfError::ResourceNotFound { name } => PyValueError::new_err(format!("Resource not found: {}", name)),
        ErfError::InvalidSignature { found } => PyValueError::new_err(format!("Invalid signature: {}", found)),
        ErfError::InvalidVersion { found } => PyValueError::new_err(format!("Invalid version: {}", found)),
        ErfError::SecurityViolation { message } => PyRuntimeError::new_err(format!("Security violation: {}", message)),
        _ => PyRuntimeError::new_err(err.to_string()),
    }
}