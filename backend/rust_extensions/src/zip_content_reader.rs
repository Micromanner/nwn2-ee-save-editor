use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyBytes};
use zip::ZipArchive;

/// Request for reading a file from ZIP
#[pyclass]
#[derive(Clone)]
pub struct ZipReadRequest {
    #[pyo3(get, set)]
    pub zip_path: String,
    #[pyo3(get, set)]
    pub internal_path: String,
    #[pyo3(get, set)]
    pub request_id: String,
}

/// Result of reading a file from ZIP
#[pyclass]
pub struct ZipReadResult {
    #[pyo3(get)]
    pub request_id: String,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub data: Option<Vec<u8>>,
    #[pyo3(get)]
    pub error: Option<String>,
}

/// Efficient ZIP content reader that keeps archives open
#[pyclass]
pub struct ZipContentReader {
    // Keep ZIP archives open for efficient batch reading
    open_archives: HashMap<String, ZipArchive<BufReader<File>>>,
    // Statistics
    files_read: u64,
    bytes_read: u64,
    archives_opened: u64,
    cache_hits: u64,
}

#[pymethods]
impl ZipContentReader {
    #[new]
    fn new() -> Self {
        ZipContentReader {
            open_archives: HashMap::new(),
            files_read: 0,
            bytes_read: 0,
            archives_opened: 0,
            cache_hits: 0,
        }
    }
    
    /// Read a single file from a ZIP archive
    fn read_file_from_zip<'a>(&mut self, py: Python<'a>, zip_path: String, internal_path: String) -> PyResult<&'a PyBytes> {
        // Check if archive is already open
        if !self.open_archives.contains_key(&zip_path) {
            self.open_archive(&zip_path)?;
        } else {
            self.cache_hits += 1;
        }
        
        // Get the archive
        let archive = self.open_archives.get_mut(&zip_path)
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyIOError, _>(
                format!("Failed to access ZIP archive: {}", zip_path)
            ))?;
        
        // Find and read the file
        match archive.by_name(&internal_path) {
            Ok(mut file) => {
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(
                        format!("Failed to read file {}: {}", internal_path, e)
                    ))?;
                
                self.files_read += 1;
                self.bytes_read += contents.len() as u64;
                Ok(PyBytes::new(py, &contents))
            }
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(
                format!("File not found in ZIP {}/{}: {}", zip_path, internal_path, e)
            ))
        }
    }
    
    /// Read multiple files in a batch (efficient for cache building)
    fn read_multiple_files(&mut self, requests: Vec<ZipReadRequest>) -> PyResult<Vec<ZipReadResult>> {
        let mut results = Vec::new();
        
        // Group requests by ZIP file for efficiency
        let mut grouped: HashMap<String, Vec<ZipReadRequest>> = HashMap::new();
        for request in requests {
            grouped.entry(request.zip_path.clone())
                .or_insert_with(Vec::new)
                .push(request);
        }
        
        // Process each ZIP file
        for (zip_path, file_requests) in grouped {
            // Open archive if needed
            if !self.open_archives.contains_key(&zip_path) {
                if let Err(e) = self.open_archive(&zip_path) {
                    // Add error results for all requests from this ZIP
                    for req in file_requests {
                        results.push(ZipReadResult {
                            request_id: req.request_id,
                            success: false,
                            data: None,
                            error: Some(format!("Failed to open ZIP: {}", e)),
                        });
                    }
                    continue;
                }
            }
            
            // Read files from this ZIP
            if let Some(archive) = self.open_archives.get_mut(&zip_path) {
                for req in file_requests {
                    match archive.by_name(&req.internal_path) {
                        Ok(mut file) => {
                            let mut contents = Vec::new();
                            match file.read_to_end(&mut contents) {
                                Ok(_) => {
                                    self.files_read += 1;
                                    self.bytes_read += contents.len() as u64;
                                    results.push(ZipReadResult {
                                        request_id: req.request_id,
                                        success: true,
                                        data: Some(contents),
                                        error: None,
                                    });
                                }
                                Err(e) => {
                                    results.push(ZipReadResult {
                                        request_id: req.request_id,
                                        success: false,
                                        data: None,
                                        error: Some(e.to_string()),
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            results.push(ZipReadResult {
                                request_id: req.request_id,
                                success: false,
                                data: None,
                                error: Some(format!("File not found: {}", e)),
                            });
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Pre-open ZIP archives for efficient access
    fn preopen_zip_archives(&mut self, zip_paths: Vec<String>) -> PyResult<()> {
        for zip_path in zip_paths {
            if !self.open_archives.contains_key(&zip_path) {
                self.open_archive(&zip_path)?;
            }
        }
        Ok(())
    }
    
    /// Close a specific archive
    fn close_archive(&mut self, zip_path: String) -> PyResult<()> {
        self.open_archives.remove(&zip_path);
        Ok(())
    }
    
    /// Close all open archives
    fn close_all_archives(&mut self) -> PyResult<()> {
        self.open_archives.clear();
        Ok(())
    }
    
    /// Get performance statistics
    fn get_stats(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("files_read", self.files_read)?;
        dict.set_item("bytes_read", self.bytes_read)?;
        dict.set_item("archives_opened", self.archives_opened)?;
        dict.set_item("cache_hits", self.cache_hits)?;
        dict.set_item("open_archives", self.open_archives.len())?;
        
        let bytes_read_mb = self.bytes_read as f64 / (1024.0 * 1024.0);
        dict.set_item("bytes_read_mb", bytes_read_mb)?;
        
        Ok(dict.into())
    }
    
    /// Check if a file exists in a ZIP without reading it
    fn file_exists_in_zip(&mut self, zip_path: String, internal_path: String) -> PyResult<bool> {
        // Open archive if needed
        if !self.open_archives.contains_key(&zip_path) {
            self.open_archive(&zip_path)?;
        }
        
        if let Some(archive) = self.open_archives.get_mut(&zip_path) {
            Ok(archive.by_name(&internal_path).is_ok())
        } else {
            Ok(false)
        }
    }
}

impl ZipContentReader {
    /// Internal method to open a ZIP archive
    fn open_archive(&mut self, zip_path: &str) -> PyResult<()> {
        let path = Path::new(zip_path);
        if !path.exists() {
            return Err(PyErr::new::<pyo3::exceptions::PyFileNotFoundError, _>(
                format!("ZIP file not found: {}", zip_path)
            ));
        }
        
        let file = File::open(path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(
                format!("Failed to open ZIP file {}: {}", zip_path, e)
            ))?;
        
        const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer
        let reader = BufReader::with_capacity(BUFFER_SIZE, file);
        
        let archive = ZipArchive::new(reader)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(
                format!("Failed to read ZIP archive {}: {}", zip_path, e)
            ))?;
        
        self.open_archives.insert(zip_path.to_string(), archive);
        self.archives_opened += 1;
        
        Ok(())
    }
}