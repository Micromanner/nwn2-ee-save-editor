pub mod erf;
pub mod tda;
pub mod tlk;
pub mod xml;
pub mod gff;

pub use erf::ErfParser;
pub use tda::TDAParser;
pub use tlk::TLKParser;
pub use xml::RustXmlParser;
pub use gff::GffParser;

#[cfg(feature = "python-bindings")]
pub use erf::PyErfParser;
#[cfg(feature = "python-bindings")]
pub use tda::PyTDAParser;
#[cfg(feature = "python-bindings")]
pub use tlk::PyTLKParser;
#[cfg(feature = "python-bindings")]
pub use xml::PyXmlParser;
#[cfg(feature = "python-bindings")]
pub use gff::python::PyGffParser;
