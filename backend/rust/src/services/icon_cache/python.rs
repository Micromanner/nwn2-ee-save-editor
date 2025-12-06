//! Python bindings for the Rust icon cache

use std::sync::Arc;
use std::path::PathBuf;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use pyo3_async_runtimes::tokio::future_into_py;

use super::{
    RustIconCache as RustIconCacheImpl,
    IconCacheConfig,
    types::SourceType,
};


/// Python wrapper for the Rust icon cache
#[pyclass(name = "RustIconCache", module = "rust_icon_cache")]
pub struct PyRustIconCache {
    inner: Arc<RustIconCacheImpl>,
}

#[pymethods]
impl PyRustIconCache {
    /// Create a new icon cache with default configuration
    #[new]
    #[pyo3(signature = (cache_directory=None, nwn2_home=None, force_rebuild=false))]
    pub fn new(cache_directory: Option<String>, nwn2_home: Option<String>, force_rebuild: bool) -> PyResult<Self> {
        let mut config = IconCacheConfig::default();
        
        if let Some(dir) = cache_directory {
            config = config.with_cache_directory(PathBuf::from(dir));
        }
        
        if force_rebuild {
            config = config.with_force_rebuild();
        }
        
        let cache = if let Some(nwn2_path) = nwn2_home {
            RustIconCacheImpl::with_nwn2_home(config, PathBuf::from(nwn2_path))
        } else {
            RustIconCacheImpl::new(config)
        };
        
        match cache {
            Ok(cache) => Ok(Self { inner: Arc::new(cache) }),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create icon cache: {}", e)
            )),
        }
    }
    
    /// Initialize the cache (async)
    pub fn initialize<'py>(&self, py: Python<'py>, force_reload: bool) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();

        future_into_py(py, async move {
            inner.initialize(force_reload)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to initialize cache: {}", e)
                ))
        })
    }
    
    /// Initialize the cache synchronously (creates its own Tokio runtime)
    pub fn initialize_sync(&self, force_reload: bool) -> PyResult<()> {
        self.inner.initialize_sync(force_reload)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to initialize cache: {}", e)
            ))
    }
    
    /// Get an icon by name (sync version)
    pub fn get_icon(&self, py: Python, name: &str) -> Option<(Py<PyBytes>, String)> {
        match self.inner.get_icon(name) {
            Some(icon) => {
                let bytes = PyBytes::new(py, &icon.data);
                Some((bytes.into(), icon.format.mime_type().to_string()))
            }
            None => None,
        }
    }
    
    /// Get an icon by name (async version)
    pub fn get_icon_async<'py>(&self, py: Python<'py>, name: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();

        future_into_py(py, async move {
            let icon = inner.get_icon_async(&name).await;

            match icon {
                Some(icon) => {
                    Python::attach(|py| {
                        let bytes = PyBytes::new(py, &icon.data);
                        let result: (Py<PyBytes>, String) = (bytes.into(), icon.format.mime_type().to_string());
                        Ok(Some(result))
                    })
                }
                None => Ok(None),
            }
        })
    }
    
    /// Get multiple icons in batch
    pub fn get_icons_batch(&self, py: Python, names: Vec<String>) -> Vec<Option<(Py<PyBytes>, String)>> {
        let icons = self.inner.get_icons_batch(&names);

        icons.into_iter()
            .map(|opt_icon| {
                opt_icon.map(|icon| {
                    let bytes = PyBytes::new(py, &icon.data);
                    (bytes.into(), icon.format.mime_type().to_string())
                })
            })
            .collect()
    }
    
    /// Set module HAKs
    pub fn set_module_haks<'py>(&self, py: Python<'py>, hak_list: Vec<String>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();

        future_into_py(py, async move {
            inner.set_module_haks(hak_list)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to set module HAKs: {}", e)
                ))
        })
    }
    
    /// Set module HAKs synchronously
    pub fn set_module_haks_sync(&self, hak_list: Vec<String>) -> PyResult<()> {
        self.inner.set_module_haks_sync(hak_list)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to set module HAKs: {}", e)
            ))
    }
    
    /// Load module icons
    pub fn load_module_icons<'py>(&self, py: Python<'py>, module_path: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        let path = PathBuf::from(module_path);

        future_into_py(py, async move {
            inner.load_module_icons(&path)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to load module icons: {}", e)
                ))
        })
    }
    
    /// Load module icons synchronously
    pub fn load_module_icons_sync(&self, module_path: String) -> PyResult<()> {
        let path = PathBuf::from(module_path);
        self.inner.load_module_icons_sync(&path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to load module icons: {}", e)
            ))
    }
    
    /// Get cache statistics
    pub fn get_statistics(&self, py: Python) -> PyResult<Py<PyAny>> {
        let stats = self.inner.get_statistics_sync();

        let dict = PyDict::new(py);
        dict.set_item("total_icons", stats.total_icons)?;
        dict.set_item("memory_usage_mb", stats.memory_usage as f64 / 1024.0 / 1024.0)?;

        // Source counts
        dict.set_item("base_count", stats.source_counts.get(&SourceType::BaseGame).unwrap_or(&0))?;
        dict.set_item("override_count", stats.source_counts.get(&SourceType::Override).unwrap_or(&0))?;
        dict.set_item("workshop_count", stats.source_counts.get(&SourceType::Workshop).unwrap_or(&0))?;
        dict.set_item("hak_count", stats.source_counts.get(&SourceType::Hak).unwrap_or(&0))?;
        dict.set_item("module_count", stats.source_counts.get(&SourceType::Module).unwrap_or(&0))?;

        dict.set_item("total_size", stats.memory_usage)?;

        Ok(dict.into())
    }
    
    /// Clear the cache and force a rebuild
    pub fn clear_cache<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();

        future_into_py(py, async move {
            // For now, just reinitialize with force_reload
            inner.initialize(true)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to clear cache: {}", e)
                ))
        })
    }
}

