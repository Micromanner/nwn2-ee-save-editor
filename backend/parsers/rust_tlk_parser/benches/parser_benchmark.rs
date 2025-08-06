use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rust_tlk_parser::TLKParser;
use std::path::PathBuf;

fn get_fixture_path(filename: &str) -> PathBuf {
    let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR should be set")
        .replace("/backend/parsers/rust_tlk_parser", "");
    PathBuf::from(format!("{}/backend/tests/fixtures/tlk/{}", workspace_root, filename))
}

fn benchmark_file_parsing(c: &mut Criterion) {
    let tlk_path = get_fixture_path("dialog_english.tlk");
    if !tlk_path.exists() {
        println!("Skipping benchmark - fixture file not found: {:?}", tlk_path);
        return;
    }

    c.bench_function("parse_tlk_file", |b| {
        b.iter(|| {
            let mut parser = TLKParser::new();
            parser.parse_from_file(black_box(&tlk_path)).unwrap();
            black_box(parser);
        })
    });
}

fn benchmark_string_access(c: &mut Criterion) {
    let tlk_path = get_fixture_path("dialog_english.tlk");
    if !tlk_path.exists() {
        println!("Skipping benchmark - fixture file not found: {:?}", tlk_path);
        return;
    }

    let mut parser = TLKParser::new();
    parser.parse_from_file(&tlk_path).unwrap();
    
    if parser.string_count() == 0 {
        return;
    }

    let max_strings = parser.string_count().min(1000);

    c.bench_function("single_string_access", |b| {
        b.iter(|| {
            for i in 0..black_box(100) {
                let str_ref = i % max_strings;
                let _ = parser.get_string(black_box(str_ref));
            }
        })
    });
}

fn benchmark_batch_operations(c: &mut Criterion) {
    let tlk_path = get_fixture_path("dialog_english.tlk");
    if !tlk_path.exists() {
        println!("Skipping benchmark - fixture file not found: {:?}", tlk_path);
        return;
    }

    let mut parser = TLKParser::new();
    parser.parse_from_file(&tlk_path).unwrap();
    
    if parser.string_count() == 0 {
        return;
    }

    let max_strings = parser.string_count().min(1000);
    
    let mut group = c.benchmark_group("batch_operations");
    
    for batch_size in [10, 50, 100, 500].iter() {
        if *batch_size > max_strings {
            continue;
        }
        
        let str_refs: Vec<usize> = (0..*batch_size).collect();
        
        group.bench_with_input(
            BenchmarkId::new("batch_string_access", batch_size),
            &str_refs,
            |b, refs| {
                b.iter(|| {
                    let result = parser.get_strings_batch(black_box(refs)).unwrap();
                    black_box(result);
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_search_operations(c: &mut Criterion) {
    let tlk_path = get_fixture_path("dialog_english.tlk");
    if !tlk_path.exists() {
        println!("Skipping benchmark - fixture file not found: {:?}", tlk_path);
        return;
    }

    let mut parser = TLKParser::new();
    parser.parse_from_file(&tlk_path).unwrap();
    
    if parser.string_count() == 0 {
        return;
    }

    let search_options = rust_tlk_parser::SearchOptions::default();

    c.bench_function("search_strings", |b| {
        b.iter(|| {
            let results = parser.search_strings(black_box("the"), black_box(&search_options)).unwrap();
            black_box(results);
        })
    });
}

fn benchmark_cache_operations(c: &mut Criterion) {
    let tlk_path = get_fixture_path("dialog_english.tlk");
    if !tlk_path.exists() {
        println!("Skipping benchmark - fixture file not found: {:?}", tlk_path);
        return;
    }

    let cache_path = std::env::temp_dir().join("benchmark_cache.tlk.cache");
    
    // Clean up any existing cache
    let _ = std::fs::remove_file(&cache_path);

    c.bench_function("save_to_cache", |b| {
        b.iter(|| {
            let mut parser = TLKParser::new();
            parser.parse_from_file(&tlk_path).unwrap();
            parser.save_to_cache(black_box(&cache_path)).unwrap();
            black_box(parser);
        })
    });

    // Ensure cache exists
    {
        let mut parser = TLKParser::new();
        parser.parse_from_file(&tlk_path).unwrap();
        parser.save_to_cache(&cache_path).unwrap();
    }

    c.bench_function("load_from_cache", |b| {
        b.iter(|| {
            let mut parser = TLKParser::new();
            let from_cache = parser.load_with_cache(black_box(&tlk_path), Some(black_box(&cache_path))).unwrap();
            black_box((parser, from_cache));
        })
    });

    // Clean up
    let _ = std::fs::remove_file(&cache_path);
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let tlk_path = get_fixture_path("dialog_english.tlk");
    if !tlk_path.exists() {
        println!("Skipping benchmark - fixture file not found: {:?}", tlk_path);
        return;
    }

    c.bench_function("memory_usage_calculation", |b| {
        let mut parser = TLKParser::new();
        parser.parse_from_file(&tlk_path).unwrap();
        
        b.iter(|| {
            let usage = parser.memory_usage();
            black_box(usage);
        })
    });
}

criterion_group!(
    benches,
    benchmark_file_parsing,
    benchmark_string_access,
    benchmark_batch_operations,
    benchmark_search_operations,
    benchmark_cache_operations,
    benchmark_memory_usage
);

criterion_main!(benches);