//! Shared DDS decoding (BC1/BC2/BC3/BC7 and legacy DXT fourcc) to RGBA8.

const DDS_MAGIC: u32 = 0x2053_4444;
const DDS_HEADER_SIZE: usize = 128;
const DDS_DX10_HEADER_SIZE: usize = 148;
const DDPF_FOURCC: u32 = 0x4;
const DDPF_RGB: u32 = 0x40;

const DXGI_FORMAT_BC1_UNORM: u32 = 71;
const DXGI_FORMAT_BC1_UNORM_SRGB: u32 = 72;
const DXGI_FORMAT_BC3_UNORM: u32 = 77;
const DXGI_FORMAT_BC3_UNORM_SRGB: u32 = 78;
const DXGI_FORMAT_BC7_UNORM: u32 = 98;
const DXGI_FORMAT_BC7_UNORM_SRGB: u32 = 99;

pub struct DecodedTexture {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

macro_rules! decode_bc {
    ($pixel_data:expr, $w:expr, $h:expr, $decoder:path, $name:literal) => {{
        let mut buf = vec![0u32; $w * $h];
        $decoder($pixel_data, $w, $h, &mut buf)
            .map_err(|e| format!(concat!($name, " decode failed: {}"), e))?;
        rgba_from_u32(&buf)
    }};
}

pub fn decode_dds_rgba(dds_bytes: &[u8]) -> Result<DecodedTexture, String> {
    if dds_bytes.len() < DDS_DX10_HEADER_SIZE {
        return Err("DDS file too small".into());
    }

    let magic = u32::from_le_bytes(dds_bytes[0..4].try_into().unwrap());
    if magic != DDS_MAGIC {
        return Err("Invalid DDS magic".into());
    }

    let height = u32::from_le_bytes(dds_bytes[12..16].try_into().unwrap()) as usize;
    let width = u32::from_le_bytes(dds_bytes[16..20].try_into().unwrap()) as usize;
    let pf_flags = u32::from_le_bytes(dds_bytes[80..84].try_into().unwrap());
    let fourcc = &dds_bytes[84..88];
    let rgb_bit_count = u32::from_le_bytes(dds_bytes[88..92].try_into().unwrap());

    let has_fourcc = pf_flags & DDPF_FOURCC != 0;

    let rgba = if has_fourcc && fourcc == b"DX10" {
        let dxgi_format = u32::from_le_bytes(dds_bytes[128..132].try_into().unwrap());
        let pixel_data = &dds_bytes[DDS_DX10_HEADER_SIZE..];

        match dxgi_format {
            DXGI_FORMAT_BC7_UNORM | DXGI_FORMAT_BC7_UNORM_SRGB => {
                decode_bc!(
                    pixel_data,
                    width,
                    height,
                    texture2ddecoder::decode_bc7,
                    "BC7"
                )
            }
            DXGI_FORMAT_BC1_UNORM | DXGI_FORMAT_BC1_UNORM_SRGB => {
                decode_bc!(
                    pixel_data,
                    width,
                    height,
                    texture2ddecoder::decode_bc1,
                    "BC1"
                )
            }
            DXGI_FORMAT_BC3_UNORM | DXGI_FORMAT_BC3_UNORM_SRGB => {
                decode_bc!(
                    pixel_data,
                    width,
                    height,
                    texture2ddecoder::decode_bc3,
                    "BC3"
                )
            }
            _ => return Err(format!("Unsupported DXGI format: {dxgi_format}")),
        }
    } else if has_fourcc {
        let pixel_data = &dds_bytes[DDS_HEADER_SIZE..];

        match fourcc {
            b"DXT1" => decode_bc!(
                pixel_data,
                width,
                height,
                texture2ddecoder::decode_bc1,
                "DXT1"
            ),
            b"DXT5" => decode_bc!(
                pixel_data,
                width,
                height,
                texture2ddecoder::decode_bc3,
                "DXT5"
            ),
            b"DXT3" => decode_bc!(
                pixel_data,
                width,
                height,
                texture2ddecoder::decode_bc2,
                "DXT3"
            ),
            _ => {
                let cc = String::from_utf8_lossy(fourcc);
                return Err(format!("Unsupported FourCC: {cc}"));
            }
        }
    } else if pf_flags & DDPF_RGB != 0 && rgb_bit_count == 32 {
        // Uncompressed A8R8G8B8: on-disk byte order is B,G,R,A. Reorder to
        // RGBA (matches three.js DDSLoader loadARGBMip) so backend capability
        // detection agrees with what the frontend renders.
        let pixel_data = &dds_bytes[DDS_HEADER_SIZE..];
        let n = width * height;
        if pixel_data.len() < n * 4 {
            return Err("Uncompressed DDS truncated".into());
        }
        let mut out = Vec::with_capacity(n * 4);
        for px in pixel_data[..n * 4].chunks_exact(4) {
            out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }
        out
    } else {
        return Err("Unsupported DDS format: no FourCC".into());
    };

    Ok(DecodedTexture {
        width,
        height,
        rgba,
    })
}

/// texture2ddecoder outputs BGRA packed in u32; convert to RGBA byte array.
fn rgba_from_u32(buf: &[u32]) -> Vec<u8> {
    buf.iter()
        .flat_map(|&pixel| {
            let b = (pixel & 0xFF) as u8;
            let g = ((pixel >> 8) & 0xFF) as u8;
            let r = ((pixel >> 16) & 0xFF) as u8;
            let a = ((pixel >> 24) & 0xFF) as u8;
            [r, g, b, a]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal legacy-DXT1 DDS: 128-byte header + one 4x4 BC1 block of
    /// solid red, padded past the 148-byte minimum the decoder enforces.
    fn synthetic_dxt1_red() -> Vec<u8> {
        let mut d = vec![0u8; 160];
        d[0..4].copy_from_slice(b"DDS ");
        d[4..8].copy_from_slice(&124u32.to_le_bytes());
        d[12..16].copy_from_slice(&4u32.to_le_bytes()); // height
        d[16..20].copy_from_slice(&4u32.to_le_bytes()); // width
        d[80..84].copy_from_slice(&0x4u32.to_le_bytes()); // DDPF_FOURCC
        d[84..88].copy_from_slice(b"DXT1");
        // BC1 block: color0 = color1 = pure red in RGB565, all indices 0
        d[128..130].copy_from_slice(&0xF800u16.to_le_bytes());
        d[130..132].copy_from_slice(&0xF800u16.to_le_bytes());
        d
    }

    /// Minimal uncompressed A8R8G8B8 DDS: 128-byte header, DDPF_RGB, 32bpp,
    /// 4x4 with the first pixel B=10,G=20,R=30,A=255. Backend must reorder
    /// on-disk BGRA to RGBA (matching three.js loadARGBMip).
    fn synthetic_argb8() -> Vec<u8> {
        let mut d = vec![0u8; 128 + 4 * 16];
        d[0..4].copy_from_slice(b"DDS ");
        d[4..8].copy_from_slice(&124u32.to_le_bytes());
        d[12..16].copy_from_slice(&4u32.to_le_bytes()); // height
        d[16..20].copy_from_slice(&4u32.to_le_bytes()); // width
        d[80..84].copy_from_slice(&0x40u32.to_le_bytes()); // DDPF_RGB
        d[88..92].copy_from_slice(&32u32.to_le_bytes()); // rgbBitCount
        d[128] = 10; // First pixel on disk: B, G, R, A
        d[129] = 20;
        d[130] = 30;
        d[131] = 255;
        d
    }

    #[test]
    fn decodes_dxt1_to_rgba() {
        let tex = decode_dds_rgba(&synthetic_dxt1_red()).expect("decode");
        assert_eq!(tex.width, 4);
        assert_eq!(tex.height, 4);
        assert_eq!(tex.rgba.len(), 4 * 4 * 4);
        // First pixel: red, opaque
        assert_eq!(tex.rgba[0], 255);
        assert_eq!(tex.rgba[1], 0);
        assert_eq!(tex.rgba[2], 0);
        assert_eq!(tex.rgba[3], 255);
    }

    #[test]
    fn decodes_uncompressed_argb8_to_rgba() {
        let tex = decode_dds_rgba(&synthetic_argb8()).expect("decode");
        assert_eq!(tex.width, 4);
        assert_eq!(tex.height, 4);
        assert_eq!(tex.rgba.len(), 4 * 4 * 4);
        // First pixel: R=30, G=20, B=10, A=255
        assert_eq!(&tex.rgba[0..4], &[30, 20, 10, 255]);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut d = vec![0u8; 160];
        d[0..4].copy_from_slice(b"NOPE");
        assert!(decode_dds_rgba(&d).is_err());
    }
}
