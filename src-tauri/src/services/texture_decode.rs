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
        for &[b, g, r, a] in pixel_data[..n * 4].as_chunks::<4>().0 {
            out.extend_from_slice(&[r, g, b, a]);
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

const DDSCAPS2_CUBEMAP: u32 = 0x200;
const DDSCAPS2_VOLUME: u32 = 0x20_0000;

/// Bytes per 4x4 block of a block-compressed DDS, or `None` when the file is
/// not block-compressed.
fn dds_block_size(dds: &[u8]) -> Option<usize> {
    let pf_flags = u32::from_le_bytes(dds[80..84].try_into().unwrap());
    if pf_flags & DDPF_FOURCC == 0 {
        return None;
    }
    match &dds[84..88] {
        b"DXT1" => Some(8),
        b"DXT3" | b"DXT5" => Some(16),
        b"DX10" => {
            let dxgi = u32::from_le_bytes(dds.get(128..132)?.try_into().unwrap());
            match dxgi {
                DXGI_FORMAT_BC1_UNORM | DXGI_FORMAT_BC1_UNORM_SRGB => Some(8),
                DXGI_FORMAT_BC3_UNORM
                | DXGI_FORMAT_BC3_UNORM_SRGB
                | DXGI_FORMAT_BC7_UNORM
                | DXGI_FORMAT_BC7_UNORM_SRGB => Some(16),
                _ => None,
            }
        }
        _ => None,
    }
}

fn dds_level_bytes(width: u32, height: u32, block_size: usize) -> usize {
    (width.div_ceil(4).max(1) as usize) * (height.div_ceil(4).max(1) as usize) * block_size
}

/// Drops the top mip levels of a block-compressed 2D DDS until the largest
/// remaining level fits `max_size`, rewriting the header to match. The mip
/// chain is stored largest level first, so this is a header patch plus a
/// slice: no decoding. Returns `None` when the file is left as is (no mip
/// chain, already small enough, cube/volume map, uncompressed, or malformed).
pub fn trim_dds_mips(dds: &[u8], max_size: u32) -> Option<Vec<u8>> {
    if dds.len() < DDS_HEADER_SIZE || u32::from_le_bytes(dds[0..4].try_into().unwrap()) != DDS_MAGIC
    {
        return None;
    }
    let caps2 = u32::from_le_bytes(dds[112..116].try_into().unwrap());
    if caps2 & (DDSCAPS2_CUBEMAP | DDSCAPS2_VOLUME) != 0 {
        return None;
    }
    let block_size = dds_block_size(dds)?;
    let header_len = if &dds[84..88] == b"DX10" {
        DDS_DX10_HEADER_SIZE
    } else {
        DDS_HEADER_SIZE
    };
    let mut height = u32::from_le_bytes(dds[12..16].try_into().unwrap());
    let mut width = u32::from_le_bytes(dds[16..20].try_into().unwrap());
    let mut mip_count = u32::from_le_bytes(dds[28..32].try_into().unwrap());
    if mip_count <= 1 || width.max(height) <= max_size {
        return None;
    }

    let mut offset = header_len;
    while mip_count > 1 && width.max(height) > max_size {
        offset += dds_level_bytes(width, height, block_size);
        width = (width / 2).max(1);
        height = (height / 2).max(1);
        mip_count -= 1;
    }
    if offset > dds.len() {
        return None;
    }

    let mut out = Vec::with_capacity(header_len + dds.len() - offset);
    out.extend_from_slice(&dds[..header_len]);
    out[12..16].copy_from_slice(&height.to_le_bytes());
    out[16..20].copy_from_slice(&width.to_le_bytes());
    out[20..24].copy_from_slice(&(dds_level_bytes(width, height, block_size) as u32).to_le_bytes());
    out[28..32].copy_from_slice(&mip_count.to_le_bytes());
    out.extend_from_slice(&dds[offset..]);
    Some(out)
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

    /// 16x16 DXT5 with a full 5-level mip chain (16, 8, 4, 2, 1); every level
    /// is filled with its own index so the trimmed data can be identified.
    fn synthetic_dxt5_mipped() -> Vec<u8> {
        let mut d = vec![0u8; 128];
        d[0..4].copy_from_slice(b"DDS ");
        d[4..8].copy_from_slice(&124u32.to_le_bytes());
        d[12..16].copy_from_slice(&16u32.to_le_bytes());
        d[16..20].copy_from_slice(&16u32.to_le_bytes());
        d[28..32].copy_from_slice(&5u32.to_le_bytes());
        d[80..84].copy_from_slice(&0x4u32.to_le_bytes());
        d[84..88].copy_from_slice(b"DXT5");
        for (level, blocks) in [16usize, 4, 1, 1, 1].into_iter().enumerate() {
            d.extend(std::iter::repeat_n(level as u8, blocks * 16));
        }
        d
    }

    #[test]
    fn trims_mips_to_requested_size() {
        let trimmed = trim_dds_mips(&synthetic_dxt5_mipped(), 4).expect("trimmed");
        assert_eq!(u32::from_le_bytes(trimmed[16..20].try_into().unwrap()), 4);
        assert_eq!(u32::from_le_bytes(trimmed[12..16].try_into().unwrap()), 4);
        assert_eq!(u32::from_le_bytes(trimmed[28..32].try_into().unwrap()), 3);
        assert_eq!(u32::from_le_bytes(trimmed[20..24].try_into().unwrap()), 16);
        // Levels 2, 3, 4 remain: one block each.
        assert_eq!(
            &trimmed[128..],
            &[[2u8; 16], [3u8; 16], [4u8; 16]].concat()[..]
        );
        // The trimmed file still decodes at the new size.
        let tex = decode_dds_rgba(&trimmed).expect("decode");
        assert_eq!((tex.width, tex.height), (4, 4));
    }

    #[test]
    fn trim_leaves_small_or_unmipped_textures_alone() {
        assert!(trim_dds_mips(&synthetic_dxt5_mipped(), 16).is_none());
        assert!(trim_dds_mips(&synthetic_dxt1_red(), 1).is_none());
        assert!(trim_dds_mips(&synthetic_argb8(), 1).is_none());
    }
}
