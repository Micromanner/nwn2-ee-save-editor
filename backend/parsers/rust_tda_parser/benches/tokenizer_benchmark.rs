use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_tda_parser::tokenizer::TDATokenizer;
use std::time::Duration;
use std::env;

/// Get measurement time from environment variable or use default
fn get_measurement_time(default_secs: u64) -> Duration {
    env::var("BENCHMARK_MEASUREMENT_TIME_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(default_secs))
}

/// Generate test lines for tokenizer benchmarks
fn generate_test_lines() -> Vec<String> {
    vec![
        // Simple space-separated
        "0 item_sword \"Iron Sword\" 100 normal".to_string(),
        
        // Tab-separated with empty fields
        "1\titem_shield\t\t50\tnormal".to_string(),
        
        // Mixed quotes and spaces
        r#"2 "complex item" "This is a \"quoted\" description" 200 "special type""#.to_string(),
        
        // Many columns
        (0..20).map(|i| format!("col{}", i)).collect::<Vec<_>>().join("\t"),
        
        // Long quoted string
        format!("3 long_item \"{}\" 500 rare", "x".repeat(200)),
        
        // Special characters
        "4 special_chars \"Item with: tabs\ttabs, spaces   spaces\" 75 ****".to_string(),
        
        // Empty fields in tab format
        "\t\t\t\t".to_string(),
        
        // Comment line
        "# This is a comment line that should be skipped".to_string(),
        
        // Empty line
        "".to_string(),
        
        // Very long line with many tokens
        (0..100).map(|i| format!("token{}", i)).collect::<Vec<_>>().join(" "),
    ]
}

/// Benchmark basic tokenization performance
fn bench_tokenization_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokenization_basic");
    let test_lines = generate_test_lines();
    
    for (i, line) in test_lines.iter().enumerate() {
        // Only measure throughput for non-empty lines
        if !line.is_empty() {
            group.throughput(Throughput::Bytes(line.len() as u64));
        }
        
        group.bench_with_input(
            BenchmarkId::new("tokenize_line", format!("line_{}", i)),
            line,
            |b, line| {
                b.iter(|| {
                    let mut tokenizer = TDATokenizer::new();
                    let tokens = tokenizer.tokenize_line(black_box(line)).unwrap();
                    black_box(tokens);
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark tokenization with different line formats
fn bench_tokenization_formats(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokenization_formats");
    
    // Space-separated format
    let space_line = "0 item1 item2 item3 item4 item5 item6 item7 item8";
    group.bench_function("space_separated", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            let tokens = tokenizer.tokenize_line(black_box(space_line)).unwrap();
            black_box(tokens);
        })
    });
    
    // Tab-separated format
    let tab_line = "0\titem1\titem2\titem3\titem4\titem5\titem6\titem7\titem8";
    group.bench_function("tab_separated", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            let tokens = tokenizer.tokenize_line(black_box(tab_line)).unwrap();
            black_box(tokens);
        })
    });
    
    // Mixed quotes format
    let quoted_line = r#"0 "item 1" item2 "item 3" item4 "item 5" item6 "item 7""#;
    group.bench_function("quoted_mixed", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            let tokens = tokenizer.tokenize_line(black_box(quoted_line)).unwrap();
            black_box(tokens);
        })
    });
    
    // Empty fields in tabs
    let empty_tab_line = "0\t\titem2\t\titem4\t\titem6\t";
    group.bench_function("empty_fields_tabs", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            let tokens = tokenizer.tokenize_line(black_box(empty_tab_line)).unwrap();
            black_box(tokens);
        })
    });
    
    group.finish();
}

/// Benchmark tokenization with varying line lengths
fn bench_tokenization_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokenization_scalability");
    
    // Generate lines of different lengths
    let lengths = vec![10, 50, 100, 200, 500, 1000];
    
    for length in lengths {
        let line = (0..length)
            .map(|i| format!("token{}", i))
            .collect::<Vec<_>>()
            .join(" ");
            
        group.throughput(Throughput::Bytes(line.len() as u64));
        
        group.bench_with_input(
            BenchmarkId::new("tokens", length),
            &line,
            |b, line| {
                b.iter(|| {
                    let mut tokenizer = TDATokenizer::new();
                    let tokens = tokenizer.tokenize_line(black_box(line)).unwrap();
                    black_box(tokens);
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark quote parsing performance
fn bench_quote_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("quote_parsing");
    
    // Simple quoted string
    let simple_quoted = r#"0 "simple quote" normal"#;
    group.bench_function("simple_quotes", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            let tokens = tokenizer.tokenize_line(black_box(simple_quoted)).unwrap();
            black_box(tokens);
        })
    });
    
    // Long quoted string
    let long_quoted = format!(r#"0 "{}" normal"#, "x".repeat(500));
    group.throughput(Throughput::Bytes(long_quoted.len() as u64));
    group.bench_function("long_quotes", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            let tokens = tokenizer.tokenize_line(black_box(&long_quoted)).unwrap();
            black_box(tokens);
        })
    });
    
    // Multiple quoted strings
    let multi_quoted = r#"0 "quote1" "quote2" "quote3" "quote4" "quote5""#;
    group.bench_function("multiple_quotes", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            let tokens = tokenizer.tokenize_line(black_box(multi_quoted)).unwrap();
            black_box(tokens);
        })
    });
    
    // Adjacent quoted and unquoted
    let adjacent_quoted = r#"0 "quote"unquoted"another" normal"#;
    group.bench_function("adjacent_quotes", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            let tokens = tokenizer.tokenize_line(black_box(adjacent_quoted)).unwrap();
            black_box(tokens);
        })
    });
    
    group.finish();
}

/// Benchmark validation performance with meaningful work
fn bench_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");
    
    // Create realistic test data that requires actual validation work
    let test_lines = vec![
        "0 item_sword \"Iron Sword\" 100 normal",
        "1 item_bow \"Elven Bow\" 200 magic",
        "2 item_armor \"Chain Mail\" 300 normal",
        "3 item_shield \"Tower Shield\" 400 heavy",
        "4 item_potion \"Health Potion\" 50 consumable",
    ];
    let long_line = "x".repeat(5000);
    let max_length = 10000;
    
    group.bench_function("multi_line_validation", |b| {
        b.iter(|| {
            let tokenizer = TDATokenizer::new();
            let mut results = Vec::with_capacity(test_lines.len());
            
            for line in black_box(&test_lines) {
                let result = tokenizer.validate_line(black_box(line), max_length);
                results.push(result);
            }
            
            black_box(results);
        })
    });
    
    group.bench_function("long_line_validation", |b| {
        b.iter(|| {
            let tokenizer = TDATokenizer::new();
            let result = tokenizer.validate_line(black_box(&long_line), 20000);
            black_box(result);
        })
    });
    
    // Add a benchmark that combines validation with tokenization for realistic workload
    group.bench_function("validation_with_tokenization", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            let mut all_tokens = Vec::new();
            
            for line in black_box(&test_lines) {
                // Validate first
                let validation_result = tokenizer.validate_line(black_box(line), max_length);
                black_box(&validation_result);
                
                // Then tokenize if validation passes
                if validation_result.is_ok() {
                    if let Ok(tokens) = tokenizer.tokenize_line(black_box(line)) {
                        all_tokens.extend(tokens);
                    }
                }
            }
            
            black_box(all_tokens);
        })
    });
    
    group.finish();
}

/// Benchmark realistic NWN2 data patterns
fn bench_realistic_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic_patterns");
    
    // Typical NWN2 2DA lines
    let nwn2_lines = vec![
        "0 longsword \"Iron Longsword\" 100 weapon melee",
        "1\tshortsword\t\"Steel Shortsword\"\t80\tweapon\tmelee",
        "2\t****\t\"Unnamed Item\"\t0\t****\t****",
        "3\thealing_potion\t\"Potion of Cure Light Wounds\"\t25\tconsumable\tmagic",
        "4 magic_ring \"Ring of Protection +1\" 500 accessory \"magic item\"",
        "# Comment line - should be skipped",
        "",
        "5\tscroll_fireball\t\"Scroll of Fireball\"\t200\tscroll\t\"spell scroll\"",
    ];
    
    group.bench_function("nwn2_pattern_mixed", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            for line in &nwn2_lines {
                let tokens = tokenizer.tokenize_line(black_box(line)).unwrap();
                black_box(tokens);
            }
        })
    });
    
    // Simulate processing an entire 2DA file
    let full_file_lines: Vec<String> = (0..1000)
        .map(|i| format!("{}\titem_{}\t\"Description {}\"\t{}\ttype_{}\tcategory_{}", 
                        i, i, i, i * 10, i % 5, i % 3))
        .collect();
    
    group.throughput(Throughput::Elements(full_file_lines.len() as u64));
    group.bench_function("full_file_simulation", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            for line in &full_file_lines {
                let tokens = tokenizer.tokenize_line(black_box(line)).unwrap();
                black_box(tokens);
            }
        })
    });
    
    group.finish();
}

/// Benchmark memory allocation patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");
    
    let test_line = "0 item1 item2 item3 item4 item5 item6 item7 item8 item9 item10";
    
    // Benchmark tokenizer reuse vs recreation
    group.bench_function("tokenizer_reuse", |b| {
        let mut tokenizer = TDATokenizer::new();
        b.iter(|| {
            let tokens = tokenizer.tokenize_line(black_box(test_line)).unwrap();
            black_box(tokens);
        })
    });
    
    group.bench_function("tokenizer_recreation", |b| {
        b.iter(|| {
            let mut tokenizer = TDATokenizer::new();
            let tokens = tokenizer.tokenize_line(black_box(test_line)).unwrap();
            black_box(tokens);
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_tokenization_basic,
    bench_tokenization_formats,
    bench_tokenization_scalability,
    bench_quote_parsing,
    bench_validation,
    bench_realistic_patterns,
    bench_memory_patterns
);

criterion_main!(benches);