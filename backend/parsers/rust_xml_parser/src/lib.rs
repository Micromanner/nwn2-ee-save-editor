pub mod types;
pub mod parser;
pub mod python;

use pyo3::prelude::*;

#[pymodule]
fn rust_xml_parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<python::PyXmlParser>()?;
    Ok(())
}
