use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_tda_parser::{SecurityLimits, TDAParser};
use std::time::Duration;
use std::fs;
use std::path::Path;
use std::env;

/// Get measurement time from environment variable or use default
fn get_measurement_time(default_secs: u64) -> Duration {
    static CACHED_TIME: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    
    let cached_secs = CACHED_TIME.get_or_init(|| {
        env::var("BENCHMARK_MEASUREMENT_TIME_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
    });
    
    Duration::from_secs(cached_secs.unwrap_or(default_secs))
}

/// Load real 2DA test files from fixtures
fn load_2da_fixtures() -> Vec<(String, String)> {
    let fixtures_path = Path::new("../../tests/fixtures/2da");
    let mut files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(fixtures_path) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext == "2da" || ext == "2DA" {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        let filename = entry.file_name().to_string_lossy().to_string();
                        files.push((filename, content));
                    }
                }
            }
        }
    }
    
    // Fallback to some basic files if directory doesn't exist
    if files.is_empty() {
        files.push(("classes.2da".to_string(), 
            "2DA V2.0\n\nLabel Name\n0 Fighter\n1 Wizard\n2 Rogue\n".to_string()));
    }
    
    files
}

/// Benchmark basic parsing performance
fn bench_parsing_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing_performance");
    group.measurement_time(get_measurement_time(3));
    
    let fixtures = load_2da_fixtures();
    
    // Test with real 2DA files, limited to first 5 for speed
    for (filename, data) in fixtures.iter().take(5) {
        let size_kb = data.len() / 1024;
        
        group.throughput(Throughput::Bytes(data.len() as u64));
        
        group.bench_with_input(
            BenchmarkId::new("parse_real_2da", format!("{}_{}KB", filename, size_kb)),
            data,
            |b, data| {
                b.iter(|| {
                    let mut parser = TDAParser::new();
                    parser.parse_from_string(black_box(data)).unwrap();
                    black_box(parser);
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark memory usage with different data sizes
fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");
    group.measurement_time(get_measurement_time(2));
    
    let fixtures = load_2da_fixtures();
    if let Some((_, test_data)) = fixtures.first() {
        group.bench_function("memory_usage_tracking", |b| {
            b.iter(|| {
                let mut parser = TDAParser::new();
                parser.parse_from_string(black_box(test_data)).unwrap();
                let memory_usage = parser.memory_usage();
                black_box(memory_usage);
            })
        });
    }
    
    group.finish();
}

/// Benchmark data access patterns
fn bench_data_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_access");
    group.measurement_time(get_measurement_time(2));
    
    // Prepare test data from fixtures
    let fixtures = load_2da_fixtures();
    if fixtures.is_empty() { return; }
    
    let test_data = &fixtures[0].1;
    let mut parser = TDAParser::new();
    parser.parse_from_string(test_data).unwrap();
    
    let row_count = parser.row_count().min(50); // Limit iterations
    let col_count = parser.column_count().min(5);
    
    group.bench_function("cell_access_by_index", |b| {
        b.iter(|| {
            for row in 0..row_count {
                for col in 0..col_count {
                    let _value = parser.get_cell(black_box(row), black_box(col));
                }
            }
        })
    });
    
    if parser.column_count() > 0 {
        let first_col_name = parser.column_names()[0].clone();
        group.bench_function("cell_access_by_name", |b| {
            b.iter(|| {
                for row in 0..row_count {
                    let _value = parser.get_cell_by_name(black_box(row), black_box(&first_col_name));
                }
            })
        });
    }
    
    group.bench_function("row_dict_access", |b| {
        b.iter(|| {
            for row in 0..row_count {
                let _dict = parser.get_row_dict(black_box(row));
            }
        })
    });

    // Safe iterator-based bulk access - more realistic workload
    group.bench_function("iterator_row_access", |b| {
        b.iter(|| {
            for row_iter in parser.iter_rows() {
                // Pass the result to black_box to prevent optimization
                black_box(row_iter.collect::<Vec<_>>());
            }
        })
    });

    if parser.column_count() > 0 {
        group.bench_function("iterator_column_access", |b| {
            b.iter(|| {
                let values = parser.iter_column(0).collect::<Vec<_>>();
                black_box(values);
            })
        });
        
        // Pure iteration benchmarks without allocation overhead
        group.bench_function("iterator_column_pure", |b| {
            b.iter(|| {
                parser.iter_column(0).for_each(|cell| {
                    black_box(cell);
                });
            })
        });
    }

    group.bench_function("iterator_row_pure", |b| {
        b.iter(|| {
            for row_iter in parser.iter_rows() {
                row_iter.for_each(|cell| {
                    black_box(cell);
                });
            }
        })
    });
    
    group.finish();
}

/// Benchmark serialization performance
fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");
    group.measurement_time(get_measurement_time(2));
    
    let fixtures = load_2da_fixtures();
    if fixtures.is_empty() { return; }
    
    let test_data = &fixtures[0].1;
    let mut parser = TDAParser::new();
    parser.parse_from_string(test_data).unwrap();
    
    group.bench_function("msgpack_serialize", |b| {
        b.iter(|| {
            let _serialized = parser.to_msgpack_compressed().unwrap();
        })
    });
    
    // Prepare serialized data for deserialization benchmark
    let serialized_data = parser.to_msgpack_compressed().unwrap();
    
    group.bench_function("msgpack_deserialize", |b| {
        b.iter(|| {
            let _parser = TDAParser::from_msgpack_compressed(black_box(&serialized_data)).unwrap();
        })
    });
    
    group.finish();
}

/// Benchmark parallel loading (simulated)
fn bench_parallel_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_loading");
    group.measurement_time(get_measurement_time(2));
    
    // Use real fixtures instead of generated data
    let fixtures = load_2da_fixtures();
    let datasets: Vec<&String> = fixtures.iter().take(4).map(|(_, data)| data).collect();
    
    group.bench_function("sequential_parsing", |b| {
        b.iter(|| {
            let mut parsers = Vec::new();
            for data in &datasets {
                let mut parser = TDAParser::new();
                parser.parse_from_string(black_box(*data)).unwrap();
                parsers.push(parser);
            }
            black_box(parsers);
        })
    });
    
    group.finish();
}

/// Benchmark security validation overhead
fn bench_security_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("security_validation");
    group.measurement_time(get_measurement_time(2));
    
    let fixtures = load_2da_fixtures();
    if fixtures.is_empty() { return; }
    let test_data = &fixtures[0].1;
    
    // Benchmark with security limits
    group.bench_function("with_security_limits", |b| {
        b.iter(|| {
            let limits = SecurityLimits::default();
            let mut parser = TDAParser::with_limits(limits);
            parser.parse_from_string(black_box(test_data)).unwrap();
            black_box(parser);
        })
    });
    
    // Benchmark with relaxed limits for comparison
    group.bench_function("with_relaxed_limits", |b| {
        b.iter(|| {
            let limits = SecurityLimits {
                max_file_size: usize::MAX,
                max_columns: usize::MAX,
                max_rows: usize::MAX,
                max_line_length: usize::MAX,
            };
            let mut parser = TDAParser::with_limits(limits);
            parser.parse_from_string(black_box(test_data)).unwrap();
            black_box(parser);
        })
    });
    
    group.finish();
}

/// Benchmark real-world-like data with various patterns
fn bench_realistic_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic_data");
    group.measurement_time(get_measurement_time(2));
    
    let fixtures = load_2da_fixtures();
    
    // Test with multiple real 2DA files to show variety
    for (filename, data) in fixtures.iter().take(3) {
        group.bench_function(&format!("real_{}", filename), |b| {
            b.iter(|| {
                let mut parser = TDAParser::new();
                parser.parse_from_string(black_box(data)).unwrap();
                
                // Simulate typical access patterns
                let _stats = parser.statistics();
                let _row_count = parser.row_count();
                if parser.row_count() > 0 {
                    let _first_row = parser.get_row_dict(0);
                }
                
                black_box(parser);
            })
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_parsing_performance,
    bench_memory_efficiency,
    bench_data_access,
    bench_serialization,
    bench_parallel_loading,
    bench_security_validation,
    bench_realistic_data
);

criterion_main!(benches);