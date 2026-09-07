use std::io::BufReader;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use super::zip_content_reader::ZipEntryLocation;

pub struct ZipEntry {
    pub stem: String,
    pub extension: String,
    pub internal_path: String,
    pub zip_path: PathBuf,
    pub size: u64,
    /// Direct-read location, so resource loads never reopen the archive.
    pub location: Option<ZipEntryLocation>,
}

/// Scan a single zip file, returning metadata for every file entry.
pub fn scan_zip(zip_path: &Path) -> Result<Vec<ZipEntry>, String> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| format!("Failed to open {}: {e}", zip_path.display()))?;

    let reader = BufReader::with_capacity(64 * 1024, file);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| format!("Failed to read zip {}: {e}", zip_path.display()))?;

    let mut entries = Vec::with_capacity(archive.len());

    for i in 0..archive.len() {
        let Ok(file) = archive.by_index_raw(i) else {
            continue;
        };

        let name = file.name().to_string();
        let p = Path::new(&name);

        let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(ext) = p.extension().and_then(|s| s.to_str()) else {
            continue;
        };

        entries.push(ZipEntry {
            stem: stem.to_lowercase(),
            extension: ext.to_lowercase(),
            internal_path: name,
            zip_path: zip_path.to_path_buf(),
            size: file.size(),
            location: ZipEntryLocation::from_raw_entry(&file),
        });
    }

    Ok(entries)
}

/// Scan multiple zip files in parallel using rayon.
pub fn scan_zips_parallel(zip_paths: &[PathBuf]) -> Result<Vec<ZipEntry>, String> {
    let results: Vec<Result<Vec<ZipEntry>, String>> =
        zip_paths.par_iter().map(|p| scan_zip(p)).collect();

    let mut all = Vec::new();
    for result in results {
        match result {
            Ok(entries) => all.extend(entries),
            Err(e) => tracing::warn!("Failed to scan zip: {e}"),
        }
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_zip(path: &Path, files: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, data) in files {
            zip.start_file(*name, options).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn test_scan_zip_indexes_all_files() {
        let temp = tempfile::TempDir::new().unwrap();
        let zip_path = temp.path().join("test.zip");
        create_test_zip(
            &zip_path,
            &[
                ("classes.2da", b"2DA V2.0"),
                ("fireball.dds", b"DDS data"),
                ("sword.uti", b"GFF data"),
            ],
        );

        let entries = scan_zip(&zip_path).unwrap();
        assert_eq!(entries.len(), 3);

        let tda = entries.iter().find(|e| e.stem == "classes").unwrap();
        assert_eq!(tda.extension, "2da");
        assert_eq!(tda.internal_path, "classes.2da");
    }

    #[test]
    fn test_scan_zip_lowercases() {
        let temp = tempfile::TempDir::new().unwrap();
        let zip_path = temp.path().join("test.zip");
        create_test_zip(&zip_path, &[("MyIcon.DDS", b"data")]);

        let entries = scan_zip(&zip_path).unwrap();
        assert_eq!(entries[0].stem, "myicon");
        assert_eq!(entries[0].extension, "dds");
        assert_eq!(entries[0].internal_path, "MyIcon.DDS");
    }

    #[test]
    fn test_scan_zip_handles_subdirs() {
        let temp = tempfile::TempDir::new().unwrap();
        let zip_path = temp.path().join("test.zip");
        create_test_zip(&zip_path, &[("subdir/nested.2da", b"2DA")]);

        let entries = scan_zip(&zip_path).unwrap();
        assert_eq!(entries[0].stem, "nested");
        assert_eq!(entries[0].internal_path, "subdir/nested.2da");
    }

    #[test]
    fn test_scan_zips_parallel() {
        let temp = tempfile::TempDir::new().unwrap();
        let zip1 = temp.path().join("a.zip");
        let zip2 = temp.path().join("b.zip");
        create_test_zip(&zip1, &[("one.2da", b"2DA")]);
        create_test_zip(&zip2, &[("two.dds", b"DDS")]);

        let entries = scan_zips_parallel(&[zip1, zip2]).unwrap();
        assert_eq!(entries.len(), 2);
    }
}
