use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rust_erf_parser::parser::ErfParser;
use rust_erf_parser::types::SecurityLimits;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn create_test_erf(num_resources: u32) -> Vec<u8> {
    let mut data = Vec::new();
    
    // Header
    data.extend_from_slice(b"HAK ");  // Signature
    data.extend_from_slice(b"V1.0");  // Version
    data.extend_from_slice(&0u32.to_le_bytes());  // Language count
    data.extend_from_slice(&0u32.to_le_bytes());  // Localized string size
    data.extend_from_slice(&num_resources.to_le_bytes());  // Entry count
    data.extend_from_slice(&0u32.to_le_bytes());  // Offset to localized string
    data.extend_from_slice(&160u32.to_le_bytes());  // Offset to key list
    let key_list_size = num_resources * 24;  // V1.0 has 24-byte entries
    let resource_list_offset = 160 + key_list_size;
    data.extend_from_slice(&resource_list_offset.to_le_bytes());  // Offset to resource list
    data.extend_from_slice(&2024u32.to_le_bytes());  // Build year - 1900
    data.extend_from_slice(&100u32.to_le_bytes());  // Build day
    data.extend_from_slice(&0u32.to_le_bytes());  // Description str ref
    
    // Reserved bytes (116 bytes)
    data.resize(160, 0);
    
    // Key list
    for i in 0..num_resources {
        let name = format!("test{:04}", i);
        let mut name_bytes = name.as_bytes().to_vec();
        name_bytes.resize(16, 0);  // V1.0 uses 16-byte names
        data.extend_from_slice(&name_bytes);
        data.extend_from_slice(&i.to_le_bytes());  // Resource ID
        data.extend_from_slice(&2017u16.to_le_bytes());  // Type (2DA)
        data.extend_from_slice(&0u16.to_le_bytes());  // Reserved
    }
    
    // Resource list
    let data_offset = resource_list_offset + (num_resources * 8);
    for i in 0..num_resources {
        let offset = data_offset + (i * 100);  // Each resource is 100 bytes
        data.extend_from_slice(&offset.to_le_bytes());
        data.extend_from_slice(&100u32.to_le_bytes());  // Size
    }
    
    // Resource data
    for _ in 0..num_resources {
        data.extend_from_slice(&[0xAA; 100]);  // Dummy data
    }
    
    data
}

fn benchmark_parse_small(c: &mut Criterion) {
    let erf_data = create_test_erf(10);
    
    c.bench_function("parse_small_erf", |b| {
        b.iter(|| {
            let mut parser = ErfParser::new();
            parser.parse_from_bytes(black_box(&erf_data)).unwrap();
        });
    });
}

fn benchmark_parse_medium(c: &mut Criterion) {
    let erf_data = create_test_erf(100);
    
    c.bench_function("parse_medium_erf", |b| {
        b.iter(|| {
            let mut parser = ErfParser::new();
            parser.parse_from_bytes(black_box(&erf_data)).unwrap();
        });
    });
}

fn benchmark_parse_large(c: &mut Criterion) {
    let erf_data = create_test_erf(1000);
    
    c.bench_function("parse_large_erf", |b| {
        b.iter(|| {
            let mut parser = ErfParser::new();
            parser.parse_from_bytes(black_box(&erf_data)).unwrap();
        });
    });
}

fn benchmark_resource_extraction(c: &mut Criterion) {
    let erf_data = create_test_erf(100);
    let mut parser = ErfParser::new();
    parser.parse_from_bytes(&erf_data).unwrap();
    
    c.bench_function("extract_single_resource", |b| {
        b.iter(|| {
            parser.extract_resource(black_box("test0050.2da")).unwrap();
            parser.clear_cache();  // Clear cache to test actual extraction
        });
    });
}

fn benchmark_list_resources(c: &mut Criterion) {
    let erf_data = create_test_erf(1000);
    let mut parser = ErfParser::new();
    parser.parse_from_bytes(&erf_data).unwrap();
    
    c.bench_function("list_all_resources", |b| {
        b.iter(|| {
            parser.list_resources(black_box(None));
        });
    });
    
    c.bench_function("list_filtered_resources", |b| {
        b.iter(|| {
            parser.list_resources(black_box(Some(2017)));  // Filter by 2DA type
        });
    });
}

fn benchmark_batch_extraction(c: &mut Criterion) {
    let erf_data = create_test_erf(50);
    
    c.bench_function("extract_all_2da", |b| {
        let dir = tempdir().unwrap();
        b.iter(|| {
            let mut parser = ErfParser::new();
            parser.parse_from_bytes(&erf_data).unwrap();
            parser.extract_all_2da(black_box(dir.path())).unwrap();
        });
    });
}

fn benchmark_parse_with_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_various_sizes");
    
    for size in [10, 50, 100, 500, 1000].iter() {
        let erf_data = create_test_erf(*size);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &erf_data,
            |b, data| {
                b.iter(|| {
                    let mut parser = ErfParser::new();
                    parser.parse_from_bytes(black_box(data)).unwrap();
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_parse_small,
    benchmark_parse_medium,
    benchmark_parse_large,
    benchmark_resource_extraction,
    benchmark_list_resources,
    benchmark_batch_extraction,
    benchmark_parse_with_sizes
);

criterion_main!(benches);