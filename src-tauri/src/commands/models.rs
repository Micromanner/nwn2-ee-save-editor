use image::{ImageBuffer, RgbaImage};
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, error, info};

use crate::services::model_loader::{self, ModelData};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub filename: String,
    pub resref: String,
    pub zip_source: String,
}

fn strip_extension(name: &str) -> &str {
    if let Some(dot) = name.rfind('.') {
        &name[..dot]
    } else {
        name
    }
}

#[tauri::command]
pub fn load_model(state: State<'_, AppState>, resref: String) -> Result<ModelData, String> {
    info!("Loading model: {}", resref);
    let rm = state.resource_manager.blocking_read();
    match model_loader::load_model(&rm, &resref, "none", "none") {
        Ok(data) => {
            info!(
                "Model loaded: {} meshes, {} hooks, skeleton={}",
                data.meshes.len(),
                data.hooks.len(),
                data.skeleton.is_some()
            );
            for mesh in &data.meshes {
                debug!(
                    "  Mesh '{}' ({}): {} verts, {} indices, diffuse='{}'",
                    mesh.name,
                    mesh.mesh_type,
                    mesh.positions.len() / 3,
                    mesh.indices.len(),
                    mesh.material.diffuse_map
                );
            }
            Ok(data)
        }
        Err(e) => {
            error!("Failed to load model '{}': {}", resref, e);
            Err(e)
        }
    }
}

#[tauri::command]
pub fn get_texture_bytes(
    state: State<'_, AppState>,
    name: String,
) -> Result<tauri::ipc::Response, String> {
    let rm = state.resource_manager.blocking_read();
    match rm.get_resource_bytes(&name, "dds") {
        Ok(bytes) => Ok(tauri::ipc::Response::new(bytes)),
        Err(e) => {
            error!("Texture not found '{}': {}", name, e);
            Err(format!("Texture not found {name}: {e}"))
        }
    }
}

#[tauri::command]
pub fn get_icon_png(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let rm = state.resource_manager.blocking_read();

    // 1. Check indexed icon files (upscaled DDS, workshop overrides)
    if let Some(icon_path) = rm.get_icon_path(&name) {
        let path: &std::path::Path = &icon_path;
        let bytes = std::fs::read(path).map_err(|e| format!("Failed to read icon {name}: {e}"))?;

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("dds")
            .to_lowercase();

        let png_bytes = if ext == "tga" {
            decode_tga_to_png(&bytes)
                .map_err(|e| format!("Failed to decode TGA icon {name}: {e}"))?
        } else {
            decode_dds_to_png(&bytes)
                .map_err(|e| format!("Failed to decode DDS icon {name}: {e}"))?
        };
        return encode_png_data_url(&png_bytes);
    }

    // 2. Fallback to get_resource_bytes (HAKs, override, zips)
    if let Ok(dds_bytes) = rm.get_resource_bytes(&name, "dds") {
        let png_bytes = decode_dds_to_png(&dds_bytes)
            .map_err(|e| format!("Failed to decode icon {name}: {e}"))?;
        return encode_png_data_url(&png_bytes);
    }

    if let Ok(tga_bytes) = rm.get_resource_bytes(&name, "tga") {
        let png_bytes = decode_tga_to_png(&tga_bytes)
            .map_err(|e| format!("Failed to decode TGA icon {name}: {e}"))?;
        return encode_png_data_url(&png_bytes);
    }

    Err(format!("Icon not found: {name}"))
}

fn encode_png_data_url(png_bytes: &[u8]) -> Result<String, String> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    Ok(format!("data:image/png;base64,{b64}"))
}

fn decode_tga_to_png(tga_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory_with_format(tga_bytes, image::ImageFormat::Tga)
        .map_err(|e| format!("TGA decode failed: {e}"))?;
    let mut png_buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png_buf, image::ImageFormat::Png)
        .map_err(|e| format!("PNG encode failed: {e}"))?;
    Ok(png_buf.into_inner())
}

fn decode_dds_to_png(dds_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let tex = crate::services::texture_decode::decode_dds_rgba(dds_bytes)?;
    let img: RgbaImage = ImageBuffer::from_raw(tex.width as u32, tex.height as u32, tex.rgba)
        .ok_or("Failed to create image buffer")?;

    let mut png_buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png_buf, image::ImageFormat::Png)
        .map_err(|e| format!("PNG encode failed: {e}"))?;

    Ok(png_buf.into_inner())
}

#[tauri::command]
pub fn list_available_models(state: State<'_, AppState>) -> Result<Vec<ModelEntry>, String> {
    let mut cache = state.model_list_cache.lock();
    if let Some(cached) = cache.as_ref() {
        info!("Returning {} cached MDB models", cached.len());
        return Ok(cached.clone());
    }

    info!("Scanning for available models...");
    let rm = state.resource_manager.blocking_read();
    let files = rm.list_resources_by_extension("mdb");
    let count = files.len();
    let result: Vec<ModelEntry> = files
        .into_iter()
        .map(|(filename, zip_source)| {
            let basename = filename.rsplit('/').next().unwrap_or(&filename);
            let resref = strip_extension(basename).to_string();
            ModelEntry {
                filename,
                resref,
                zip_source,
            }
        })
        .collect();
    info!("Found {} MDB models", count);
    *cache = Some(result.clone());
    Ok(result)
}
