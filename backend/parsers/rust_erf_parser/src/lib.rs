pub mod error;
pub mod parser;
pub mod python;
pub mod types;

use pyo3::prelude::*;
use python::PyErfParser;

#[pymodule]
fn rust_erf_parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyErfParser>()?;
    Ok(())
}