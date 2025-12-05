use pyo3::prelude::*;
use crate::parser::{RustXmlParser};
use pythonize::pythonize;

#[pyclass(name = "XmlParser")]
pub struct PyXmlParser {
    inner: RustXmlParser,
}

#[pymethods]
impl PyXmlParser {
    #[new]
    fn new(xml_content: &str) -> PyResult<Self> {
        match RustXmlParser::from_string(xml_content) {
            Ok(parser) => Ok(PyXmlParser { inner: parser }),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(e)),
        }
    }

    fn to_xml_string(&self) -> PyResult<String> {
        self.inner.to_xml_string().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))
    }

    fn get_companion_status(&self, py: Python) -> PyResult<PyObject> {
        let status = self.inner.get_companion_status();
        Ok(pythonize(py, &status)?.into())
    }

    fn get_quest_overview(&self, py: Python) -> PyResult<PyObject> {
        let overview = self.inner.get_quest_overview_struct();
        Ok(pythonize(py, &overview)?.into())
    }

    fn get_general_info(&self, py: Python) -> PyResult<PyObject> {
        let info = self.inner.get_general_info();
        Ok(pythonize(py, &info)?.into())
    }

    fn get_full_summary(&self, py: Python) -> PyResult<PyObject> {
        let summary = self.inner.get_full_summary_struct();
        Ok(pythonize(py, &summary)?.into())
    }

    // Variable Accessors
    fn get_all_integers(&self, py: Python) -> PyResult<PyObject> {
        Ok(pythonize(py, &self.inner.data.integers)?.into())
    }

    fn get_all_strings(&self, py: Python) -> PyResult<PyObject> {
        Ok(pythonize(py, &self.inner.data.strings)?.into())
    }

    fn get_all_floats(&self, py: Python) -> PyResult<PyObject> {
        Ok(pythonize(py, &self.inner.data.floats)?.into())
    }
    
    fn get_variable(&self, py: Python, var_name: &str, var_type: &str) -> PyResult<PyObject> {
        if var_type == "int" {
             if let Some(val) = self.inner.data.integers.get(var_name) { return Ok(val.into_py(py)); }
        } else if var_type == "string" {
             if let Some(val) = self.inner.data.strings.get(var_name) { return Ok(val.into_py(py)); }
        } else if var_type == "float" {
             if let Some(val) = self.inner.data.floats.get(var_name) { return Ok(val.into_py(py)); }
        }
        Ok(py.None())
    }
    
    fn update_integer(&mut self, var_name: String, value: i32) -> bool {
        self.inner.data.integers.insert(var_name, value);
        true
    }
    
    fn update_string(&mut self, var_name: String, value: String) -> bool {
        self.inner.data.strings.insert(var_name, value);
        true
    }
    
    fn update_float(&mut self, var_name: String, value: f32) -> bool {
        self.inner.data.floats.insert(var_name, value);
        true
    }
    
    fn update_variable(&mut self, var_name: String, value: &Bound<'_, PyAny>, var_type: &str) -> PyResult<bool> {
        match var_type {
            "int" => {
                let val: i32 = value.extract()?;
                Ok(self.update_integer(var_name, val))
            },
            "string" => {
                let val: String = value.extract()?;
                Ok(self.update_string(var_name, val))
            },
            "float" => {
                let val: f32 = value.extract()?;
                Ok(self.update_float(var_name, val))
            },
            _ => Ok(false)
        }
    }
    
    fn delete_variable(&mut self, var_name: &str, var_type: &str) -> bool {
        match var_type {
            "int" => self.inner.data.integers.remove(var_name).is_some(),
            "string" => self.inner.data.strings.remove(var_name).is_some(),
            "float" => self.inner.data.floats.remove(var_name).is_some(),
            "vector" => self.inner.data.vectors.remove(var_name).is_some(),
            _ => false
        }
    }

    fn update_companion_influence(&mut self, companion_id: &str, new_influence: i32) -> bool {
        let definitions = crate::parser::get_companion_definitions();
        if let Some(def) = definitions.get(companion_id) {
            self.inner.data.integers.insert(def.influence_var.to_string(), new_influence);
            return true;
        }
        
        let discovered = self.inner.discover_potential_companions();
        if discovered.contains_key(companion_id) {
             let pattern = format!(r"(?i)^(?:[a-zA-Z0-9_]*_)?(?:inf|influence){}$", companion_id);
             let regex = regex::Regex::new(&pattern).unwrap();
             
             let keys: Vec<String> = self.inner.data.integers.keys().cloned().collect();
             for key in keys {
                 if regex.is_match(&key) {
                     self.inner.data.integers.insert(key, new_influence);
                     return true;
                 }
             }
        }
        
        false
    }
}
