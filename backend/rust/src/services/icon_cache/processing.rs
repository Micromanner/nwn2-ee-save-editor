//! Image processing pipeline for icon conversion

use std::path::{Path, PathBuf};
use std::io::Cursor;
use image::{DynamicImage, ImageFormat as ImgFormat, RgbaImage};
use rayon::prelude::*;
use tokio::task;
use ddsfile::{Dds, DxgiFormat};
// No changes needed for image_dds import, it's used as a crate.

use super::{
    types::{CachedIcon, IconSource, SourceType, ImageFormat, InputFormat},
    config::IconCacheConfig,
    error::{Result, IconCacheError},
};

/// Handles image processing and format conversion
pub struct ImageProcessor {
    config: IconCacheConfig,
}

impl ImageProcessor {
    /// Create a new image processor
    pub fn new(config: IconCacheConfig) -> Self {
        Self { config }
    }
    
    /// Process an icon source and return processed icons
    pub async fn process_source(
        &self,
        source: IconSource,
    ) -> Result<Vec<(String, CachedIcon)>> {
        let config = self.config.clone();
        
        // Handle different source types
        match source.source_type {
            SourceType::Hak => self.process_hak_source(source).await,
            _ => self.process_regular_source(source, config).await,
        }
    }
    
    /// Process a regular directory source
    async fn process_regular_source(
        &self,
        source: IconSource,
        config: IconCacheConfig,
    ) -> Result<Vec<(String, CachedIcon)>> {
        let source_type = source.source_type;
        let base_path = source.path.clone();
        
        // Process icons in parallel using rayon
        let processed = task::spawn_blocking(move || {
            let results: Vec<_> = source.icons.par_iter()
                .map(|icon_name| {
                    match process_single_icon(&base_path, icon_name, source_type, &config) {
                        Ok(icon) => Ok((icon_name.clone(), icon)),
                        Err(e) => {
                            // Log different error levels based on error type
                            let error_str = e.to_string();
                            if error_str.contains("BC7 decompression failed") {
                                log::warn!("BC7 decompression failed for icon '{}': {}", icon_name, e);
                            } else if error_str.contains("Icon not found") {
                                log::trace!("Icon not found: {}", icon_name);
                            } else {
                                log::debug!("Failed to process icon '{}': {}", icon_name, e);
                            }
                            Err(e)
                        }
                    }
                })
                .collect();
            
            // Count successes and failures
            let (successes, failures): (Vec<_>, Vec<_>) = results.into_iter()
                .partition(Result::is_ok);
            
            let success_count = successes.len();
            let failure_count = failures.len();
            
            if failure_count > 0 {
                log::warn!("Failed to process {} out of {} icons", failure_count, success_count + failure_count);
            }
            
            // Return only the successful ones
            successes.into_iter()
                .filter_map(Result::ok)
                .collect::<Vec<_>>()
        }).await
        .map_err(|e| IconCacheError::RuntimeError(e.to_string()))?;
        
        Ok(processed)
    }
    
    /// Process a HAK file source
    async fn process_hak_source(
        &self,
        source: IconSource,
    ) -> Result<Vec<(String, CachedIcon)>> {
        // For now, we'll skip HAK processing
        // In a full implementation, we'd use an ERF parser here
        log::warn!("HAK file processing not yet implemented: {:?}", source.path);
        Ok(vec![])
    }
    
    /// Convert raw image data to optimized format
    pub fn convert_image_data(
        &self,
        data: &[u8],
        input_format: InputFormat,
    ) -> Result<Vec<u8>> {
        // Load the image
        let img = match input_format {
            InputFormat::Tga => {
                // TGA support through image crate
                image::load_from_memory_with_format(data, ImgFormat::Tga)?
            }
            InputFormat::Dds => {
                // Special handling for DDS
                let rgba_img = dds_to_rgba(data)?;
                DynamicImage::ImageRgba8(rgba_img)
            }
            InputFormat::Png => {
                image::load_from_memory_with_format(data, ImgFormat::Png)?
            }
            InputFormat::Unknown => {
                // Try to detect format
                image::load_from_memory(data)?
            }
        };
        
        // Convert to RGBA if needed
        let img = if let DynamicImage::ImageRgba8(_) = img {
            img
        } else {
            DynamicImage::ImageRgba8(img.to_rgba8())
        };
        
        // Resize if too large
        let img = if img.width() > 64 || img.height() > 64 {
            img.thumbnail(64, 64)
        } else {
            img
        };
        
        // Convert to WebP format
        let mut output = Vec::new();
        let mut cursor = Cursor::new(&mut output);
        img.write_to(&mut cursor, ImgFormat::WebP)?;
        Ok(output)
    }
}

/// Process a single icon file
fn process_single_icon(
    base_path: &Path,
    icon_name: &str,
    source_type: SourceType,
    config: &IconCacheConfig,
) -> Result<CachedIcon> {
    // Icon name includes the relative path (e.g., "ui/upscaled/icons/myicon")
    // We need to try different extensions
    let extensions = ["dds", "tga", "png", "DDS", "TGA", "PNG"];
    
    // Log what we're looking for
    log::debug!("Looking for icon '{}' in base path: {:?}", icon_name, base_path);
    
    for ext in &extensions {
        let file_path = base_path.join(format!("{}.{}", icon_name, ext));
        log::trace!("Trying path: {:?}", file_path);
        
        if file_path.exists() {
            log::debug!("Found icon file: {:?}", file_path);
            match process_icon_file(&file_path, source_type, config) {
                Ok(icon) => return Ok(icon),
                Err(e) => {
                    log::warn!("Failed to process icon file {:?}: {}", file_path, e);
                    // Continue trying other formats
                }
            }
        }
    }
    
    // Also try without adding extension (in case icon_name already has it)
    let direct_path = base_path.join(icon_name);
    if direct_path.exists() {
        log::debug!("Found icon file (direct path): {:?}", direct_path);
        return process_icon_file(&direct_path, source_type, config);
    }
    
    log::trace!("Icon not found: {} in {:?}", icon_name, base_path);
    Err(IconCacheError::IconNotFound(icon_name.to_string()))
}

/// Convert DDS to RGBA image
fn dds_to_rgba(data: &[u8]) -> Result<RgbaImage> {
    log::trace!("Parsing DDS file, {} bytes", data.len());
    
    let dds = Dds::read(&mut Cursor::new(data))
        .map_err(|e| IconCacheError::Other(format!("Failed to parse DDS: {}", e)))?;
    
    let width = dds.get_width();
    let height = dds.get_height();
    let format = dds.get_dxgi_format();
    
    log::trace!("DDS info: {}x{}, format: {:?}", width, height, format);
    
    // Get the raw data - DDS files can have various formats
    let raw_data = dds.get_data(0)
        .map_err(|e| IconCacheError::Other(format!("Failed to get DDS data: {}", e)))?;
    
    log::trace!("DDS raw data size: {} bytes", raw_data.len());
    
    // Convert based on format
    let rgba_data = match format {
        Some(DxgiFormat::BC1_UNorm) | Some(DxgiFormat::BC1_UNorm_sRGB) => {
            log::trace!("Decompressing BC1/DXT1 format");
            // DXT1 compression
            decompress_bc1(raw_data, width, height)?
        }
        Some(DxgiFormat::BC2_UNorm) | Some(DxgiFormat::BC2_UNorm_sRGB) => {
            log::trace!("Decompressing BC2/DXT3 format");
            // DXT3 compression
            decompress_bc2(raw_data, width, height)?
        }
        Some(DxgiFormat::BC3_UNorm) | Some(DxgiFormat::BC3_UNorm_sRGB) => {
            log::trace!("Decompressing BC3/DXT5 format");
            // DXT5 compression
            decompress_bc3(raw_data, width, height)?
        }
        Some(DxgiFormat::BC7_UNorm) | Some(DxgiFormat::BC7_UNorm_sRGB) => {
            log::debug!("Found BC7 format icon, decompressing with image_dds");
            // Use image_dds for BC7 decompression
            let result = decompress_bc7_with_image_dds(&dds, width, height)?;
            log::info!("Successfully decompressed BC7 icon");
            result
        }
        Some(DxgiFormat::R8G8B8A8_UNorm) | Some(DxgiFormat::R8G8B8A8_UNorm_sRGB) => {
            log::trace!("DDS is already RGBA format");
            // Already RGBA
            raw_data.to_vec()
        }
        _ => {
            log::trace!("Unknown DDS format, trying as uncompressed RGBA");
            // Try to handle as uncompressed RGBA
            if raw_data.len() == (width * height * 4) as usize {
                raw_data.to_vec()
            } else {
                return Err(IconCacheError::Other(format!(
                    "Unsupported DDS format: {:?}, data size: {}, expected: {}",
                    format, raw_data.len(), width * height * 4
                )));
            }
        }
    };
    
    log::trace!("Creating RGBA image from decompressed data ({} bytes)", rgba_data.len());
    
    RgbaImage::from_raw(width, height, rgba_data)
        .ok_or_else(|| IconCacheError::Other("Failed to create RGBA image from DDS".to_string()))
}

/// BC1 (DXT1) decompression
fn decompress_bc1(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let mut output = vec![0u8; (width * height * 4) as usize];
    texpresso::Format::Bc1.decompress(data, width as usize, height as usize, &mut output);
    Ok(output)
}

/// BC2 (DXT3) decompression
fn decompress_bc2(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let mut output = vec![0u8; (width * height * 4) as usize];
    texpresso::Format::Bc2.decompress(data, width as usize, height as usize, &mut output);
    Ok(output)
}

/// BC3 (DXT5) decompression
fn decompress_bc3(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let mut output = vec![0u8; (width * height * 4) as usize];
    texpresso::Format::Bc3.decompress(data, width as usize, height as usize, &mut output);
    Ok(output)
}

/// BC7 decompression using image_dds v0.5.1
fn decompress_bc7_with_image_dds(dds: &Dds, width: u32, height: u32) -> Result<Vec<u8>> {
    log::trace!("Starting BC7 decompression with image_dds v0.5.1");

    // The `image_from_dds` function returns an `ImageBuffer` from the `image` crate.
    let image = image_dds::image_from_dds(dds, 0)
        .map_err(|e| IconCacheError::Other(format!("BC7 decompression failed with image_dds: {}", e)))?;

    // Verify dimensions match what the DDS header reported. This is good practice.
    // The `ImageBuffer` type has private fields, so we must use the public
    // `width()` and `height()` accessor methods, as suggested by the compiler.
    if image.width() != width || image.height() != height {
        return Err(IconCacheError::Other(format!(
            "BC7 decompression dimension mismatch: expected {}x{}, got {}x{}",
            width, height, image.width(), image.height()
        )));
    }

    // To get the raw pixel data from an `ImageBuffer`, we use the `into_raw()` method,
    // which consumes the buffer and returns the underlying Vec<u8>.
    Ok(image.into_raw())
}



/// Process an icon file
fn process_icon_file(
    path: &Path,
    source_type: SourceType,
    _config: &IconCacheConfig,
) -> Result<CachedIcon> {
    log::trace!("Processing icon file: {:?}", path);
    
    // Read file
    let data = std::fs::read(path)?;
    log::trace!("Read {} bytes from {:?}", data.len(), path);
    
    // Detect format
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let input_format = InputFormat::from_extension(ext);
    log::trace!("Detected format: {:?} for extension: {}", input_format, ext);
    
    // Load and convert image
    let img = match input_format {
        InputFormat::Tga => {
            log::trace!("Loading TGA image");
            image::load_from_memory_with_format(&data, ImgFormat::Tga)?
        }
        InputFormat::Dds => {
            log::trace!("Loading DDS image");
            // Special handling for DDS
            let rgba_img = dds_to_rgba(&data)?;
            DynamicImage::ImageRgba8(rgba_img)
        }
        InputFormat::Png => {
            log::trace!("Loading PNG image");
            image::load_from_memory_with_format(&data, ImgFormat::Png)?
        }
        InputFormat::Unknown => {
            return Err(IconCacheError::Other(format!("Unknown format: {}", ext)));
        }
    };
    
    let (width, height) = (img.width(), img.height());
    log::trace!("Loaded image: {}x{}", width, height);
    
    // Convert to RGBA if not already
    let img = if let DynamicImage::ImageRgba8(_) = img {
        img
    } else {
        log::trace!("Converting to RGBA");
        DynamicImage::ImageRgba8(img.to_rgba8())
    };
    
    // Resize if needed (to 64x64 max)
    let img = if img.width() > 64 || img.height() > 64 {
        log::trace!("Resizing from {}x{} to thumbnail", img.width(), img.height());
        img.thumbnail(64, 64)
    } else {
        img
    };
    
    // Convert to WebP format
    log::trace!("Converting to WebP format");
    let mut output = Vec::new();
    let mut cursor = Cursor::new(&mut output);
    img.write_to(&mut cursor, ImgFormat::WebP)?;

    log::debug!("Successfully processed icon: {:?} -> {} bytes WebP", path, output.len());
    Ok(CachedIcon::new(output, ImageFormat::WebP, source_type))
}

/// Batch process multiple icons
pub async fn process_icons_batch(
    icons: Vec<(PathBuf, String, SourceType)>,
    config: &IconCacheConfig,
) -> Vec<Result<(String, CachedIcon)>> {
    let config = config.clone();
    
    task::spawn_blocking(move || {
        icons.into_par_iter()
            .map(|(path, name, source_type)| {
                process_icon_file(&path, source_type, &config)
                    .map(|icon| (name, icon))
            })
            .collect()
    }).await
    .unwrap_or_else(|e| {
        vec![Err(IconCacheError::RuntimeError(e.to_string()))]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_input_format_detection() {
        assert_eq!(InputFormat::from_extension("tga"), InputFormat::Tga);
        assert_eq!(InputFormat::from_extension("TGA"), InputFormat::Tga);
        assert_eq!(InputFormat::from_extension("dds"), InputFormat::Dds);
        assert_eq!(InputFormat::from_extension("png"), InputFormat::Png);
        assert_eq!(InputFormat::from_extension("jpg"), InputFormat::Unknown);
    }
    
    #[tokio::test]
    async fn test_image_processor() {
        let config = IconCacheConfig::default();
        let processor = ImageProcessor::new(config);
        
        // Create a simple PNG for testing
        let img = DynamicImage::ImageRgba8(image::RgbaImage::new(32, 32));
        let mut png_data = Vec::new();
        img.write_to(&mut Cursor::new(&mut png_data), ImgFormat::Png).unwrap();
        
        // Convert it
        let result = processor.convert_image_data(&png_data, InputFormat::Png).unwrap();
        
        // Should be converted to WebP or AVIF
        assert!(!result.is_empty());
    }
}
