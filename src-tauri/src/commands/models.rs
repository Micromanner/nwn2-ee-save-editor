use std::sync::Arc;

use image::{ImageBuffer, RgbaImage};
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, error, info};

use crate::services::model_loader::{self, ModelData};
use crate::services::resource_manager::ResourceManager;
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
pub async fn load_model(state: State<'_, AppState>, resref: String) -> Result<ModelData, String> {
    info!("Loading model: {}", resref);
    let rm = Arc::clone(&state.resource_manager).read_owned().await;
    tokio::task::spawn_blocking(move || {
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
    })
    .await
    .map_err(|e| format!("Model load task failed: {e}"))?
}

#[tauri::command]
pub async fn get_texture_bytes(
    state: State<'_, AppState>,
    name: String,
    max_size: Option<u32>,
) -> Result<tauri::ipc::Response, String> {
    let rm = state.resource_manager.read().await;
    match rm.get_resource_bytes(&name, "dds") {
        Ok(bytes) => {
            // Thumbnails cap the texture size: dropping the top mips of a
            // 2048px EE upscale cuts IPC, parse and GPU upload 16x.
            let bytes = max_size
                .and_then(|max| crate::services::texture_decode::trim_dds_mips(&bytes, max))
                .unwrap_or(bytes);
            Ok(tauri::ipc::Response::new(bytes))
        }
        Err(e) => {
            error!("Texture not found '{}': {}", name, e);
            Err(format!("Texture not found {name}: {e}"))
        }
    }
}

#[tauri::command]
pub async fn get_icon_png(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let rm = Arc::clone(&state.resource_manager).read_owned().await;
    tokio::task::spawn_blocking(move || icon_png_data_url(&rm, &name))
        .await
        .map_err(|e| format!("Icon task failed: {e}"))?
}

fn icon_png_data_url(rm: &ResourceManager, name: &str) -> Result<String, String> {
    // 1. Check indexed icon files (upscaled DDS, workshop overrides)
    if let Some(icon_path) = rm.get_icon_path(name) {
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
    if let Ok(dds_bytes) = rm.get_resource_bytes(name, "dds") {
        let png_bytes = decode_dds_to_png(&dds_bytes)
            .map_err(|e| format!("Failed to decode icon {name}: {e}"))?;
        return encode_png_data_url(&png_bytes);
    }

    if let Ok(tga_bytes) = rm.get_resource_bytes(name, "tga") {
        let png_bytes = decode_tga_to_png(&tga_bytes)
            .map_err(|e| format!("Failed to decode TGA icon {name}: {e}"))?;
        return encode_png_data_url(&png_bytes);
    }

    Err(format!("Icon not found: {name}"))
}

pub(crate) fn encode_png_data_url(png_bytes: &[u8]) -> Result<String, String> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    Ok(format!("data:image/png;base64,{b64}"))
}

pub(crate) fn decode_tga_to_png(tga_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory_with_format(tga_bytes, image::ImageFormat::Tga)
        .map_err(|e| format!("TGA decode failed: {e}"))?;
    let mut png_buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png_buf, image::ImageFormat::Png)
        .map_err(|e| format!("PNG encode failed: {e}"))?;
    Ok(png_buf.into_inner())
}

pub(crate) fn decode_dds_to_png(dds_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let tex = crate::services::texture_decode::decode_dds_rgba(dds_bytes)?;
    let img: RgbaImage = ImageBuffer::from_raw(tex.width as u32, tex.height as u32, tex.rgba)
        .ok_or("Failed to create image buffer")?;

    let mut png_buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png_buf, image::ImageFormat::Png)
        .map_err(|e| format!("PNG encode failed: {e}"))?;

    Ok(png_buf.into_inner())
}

#[tauri::command]
pub async fn list_available_models(state: State<'_, AppState>) -> Result<Vec<ModelEntry>, String> {
    if let Some(cached) = state.model_list_cache.lock().as_ref() {
        info!("Returning {} cached MDB models", cached.len());
        return Ok(cached.clone());
    }

    info!("Scanning for available models...");
    let rm = Arc::clone(&state.resource_manager).read_owned().await;
    let result = tokio::task::spawn_blocking(move || {
        let files = rm.list_resources_by_extension("mdb");
        info!("Found {} MDB models", files.len());
        files
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
            .collect::<Vec<ModelEntry>>()
    })
    .await
    .map_err(|e| format!("Model scan task failed: {e}"))?;
    *state.model_list_cache.lock() = Some(result.clone());
    Ok(result)
}
