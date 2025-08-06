//! Icon discovery and file scanning

use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use tokio::task;

use crate::{
    types::{IconSource, SourceType},
    error::Result,
};

/// Handles discovering icon files in the filesystem
pub struct IconDiscovery {
    /// Supported icon extensions
    supported_extensions: Vec<String>,
}

impl IconDiscovery {
    /// Create a new icon discovery engine
    pub fn new() -> Self {
        Self {
            supported_extensions: vec![
                "tga".to_string(),
                "dds".to_string(),
                "png".to_string(),
            ],
        }
    }
    
    /// Scan a directory for icons with a specific source type
    pub async fn scan_directory(
        &self,
        path: &Path,
        source_type: SourceType,
    ) -> Result<Vec<IconSource>> {
        let path = path.to_path_buf();
        let extensions = self.supported_extensions.clone();
        
        // Use spawn_blocking for CPU-intensive directory scanning
        let sources = task::spawn_blocking(move || {
            scan_directory_sync(&path, source_type, &extensions)
        }).await
        .map_err(|e| crate::error::IconCacheError::RuntimeError(e.to_string()))?;
        
        sources
    }
    
    /// Scan multiple directories in parallel
    pub async fn scan_directories(
        &self,
        paths: &[(PathBuf, SourceType)],
    ) -> Result<Vec<IconSource>> {
        let mut all_sources = Vec::new();
        
        for (path, source_type) in paths {
            let sources = self.scan_directory(path, *source_type).await?;
            all_sources.extend(sources);
        }
        
        Ok(all_sources)
    }
}

/// Synchronous directory scanning with parallel processing
fn scan_directory_sync(
    path: &Path,
    source_type: SourceType,
    extensions: &[String],
) -> Result<Vec<IconSource>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    
    // Special handling for different source types
    match source_type {
        SourceType::Workshop => scan_workshop_directory(path, extensions),
        SourceType::Hak => scan_hak_files(path, extensions),
        _ => scan_regular_directory(path, source_type, extensions),
    }
}

/// Scan a regular directory (base game, override)
fn scan_regular_directory(
    path: &Path,
    source_type: SourceType,
    extensions: &[String],
) -> Result<Vec<IconSource>> {
    let mut sources = Vec::new();
    
    // Collect all icon files with their paths
    let mut all_icons = Vec::new();
    
    for entry in WalkDir::new(path)
        .max_depth(5) // Reasonable depth limit
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        
        if extensions.contains(&ext) {
            // Store the relative path from the base path as the icon name
            // This preserves the directory structure
            if let Ok(rel_path) = file_path.strip_prefix(path) {
                // Convert path to string, removing extension
                let icon_name = rel_path
                    .to_string_lossy()
                    .replace('\\', "/") // Normalize path separators
                    .trim_end_matches(&format!(".{}", ext))
                    .to_string();
                
                log::trace!("Found icon: {} (from file: {:?})", icon_name, file_path);
                all_icons.push(icon_name);
            }
        }
    }
    
    // Create a single IconSource for all icons in this path
    if !all_icons.is_empty() {
        sources.push(IconSource {
            path: path.to_path_buf(),
            source_type,
            icons: all_icons,
        });
    }
    
    log::info!("Found {} icon sources in {:?}", sources.len(), path);
    
    Ok(sources)
}

/// Scan workshop directories (special structure)
fn scan_workshop_directory(
    path: &Path,
    extensions: &[String],
) -> Result<Vec<IconSource>> {
    let mut all_sources = Vec::new();
    
    // Workshop structure: workshop/content/2738630/[workshop_id]/override/
    for workshop_entry in std::fs::read_dir(path)? {
        let workshop_entry = workshop_entry?;
        if !workshop_entry.file_type()?.is_dir() {
            continue;
        }
        
        let workshop_path = workshop_entry.path();
        
        // Check override subdirectory
        let override_path = workshop_path.join("override");
        if override_path.exists() {
            let sources = scan_regular_directory(&override_path, SourceType::Workshop, extensions)?;
            all_sources.extend(sources);
        }
        
        // Also check root directory
        let sources = scan_regular_directory(&workshop_path, SourceType::Workshop, extensions)?;
        all_sources.extend(sources);
    }
    
    Ok(all_sources)
}

/// Scan for HAK files
fn scan_hak_files(
    path: &Path,
    _extensions: &[String],
) -> Result<Vec<IconSource>> {
    let hak_files: Vec<PathBuf> = WalkDir::new(path)
        .max_depth(2)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file() &&
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("hak"))
                .unwrap_or(false)
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();
    
    // For HAK files, we'll need special processing in the image processor
    // For now, just return them as sources
    let sources: Vec<IconSource> = hak_files.into_iter()
        .map(|hak_path| IconSource {
            path: hak_path,
            source_type: SourceType::Hak,
            icons: vec![], // Will be populated during extraction
        })
        .collect();
    
    Ok(sources)
}

/// Check if a path component matches a source type pattern
pub fn detect_source_type_from_path(path: &Path) -> SourceType {
    let path_str = path.to_string_lossy().to_lowercase();
    
    // Check for specific patterns in the path
    if path_str.contains("workshop") && path_str.contains("content") {
        SourceType::Workshop
    } else if path_str.contains("override") && !path_str.contains("workshop") {
        SourceType::Override
    } else if path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("hak"))
        .unwrap_or(false) {
        SourceType::Hak
    } else if path_str.contains("modules") && path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("mod"))
        .unwrap_or(false) {
        SourceType::Module
    } else {
        SourceType::BaseGame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[tokio::test]
    async fn test_icon_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let discovery = IconDiscovery::new();
        
        // Create some test files
        let icons_dir = temp_dir.path().join("icons");
        fs::create_dir(&icons_dir).unwrap();
        
        fs::write(icons_dir.join("test1.tga"), b"fake tga data").unwrap();
        fs::write(icons_dir.join("test2.dds"), b"fake dds data").unwrap();
        fs::write(icons_dir.join("test3.png"), b"fake png data").unwrap();
        fs::write(icons_dir.join("not_an_icon.txt"), b"text file").unwrap();
        
        // Scan directory
        let sources = discovery.scan_directory(&icons_dir, SourceType::BaseGame).await.unwrap();
        
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_type, SourceType::BaseGame);
        assert_eq!(sources[0].icons.len(), 3);
        assert!(sources[0].icons.contains(&"test1".to_string()));
        assert!(sources[0].icons.contains(&"test2".to_string()));
        assert!(sources[0].icons.contains(&"test3".to_string()));
    }
    
    #[test]
    fn test_source_type_detection() {
        let workshop_path = Path::new("/steamapps/workshop/content/2738630/123456/override");
        assert_eq!(detect_source_type_from_path(workshop_path), SourceType::Workshop);
        
        let override_path = Path::new("/Documents/NWN2/override");
        assert_eq!(detect_source_type_from_path(override_path), SourceType::Override);
        
        let hak_path = Path::new("/Documents/NWN2/hak/my_icons.hak");
        assert_eq!(detect_source_type_from_path(hak_path), SourceType::Hak);
        
        let base_path = Path::new("/NWN2/ui/upscaled/icons");
        assert_eq!(detect_source_type_from_path(base_path), SourceType::BaseGame);
    }
}