# Rust TLK Parser

A high-performance Rust parser for Neverwinter Nights 2 TLK (Talk Table) files, designed to replace the Python implementation in the NWN2 Enhanced Edition Editor.

## Performance Goals

This parser was specifically created to solve a critical performance bottleneck:
- **Problem**: 4,464 TLK string lookups taking 4.6 seconds during application startup
- **Solution**: Load all strings into memory once, then use hash map lookups
- **Target**: Reduce lookup time from 4.6s to <0.5s (10x improvement)
- **Overall Impact**: Reduce total startup time from 5.9s to ~2.8s (52% improvement)

## Features

- **Memory-optimized storage**: Load all strings into memory once for O(1) access
- **Bulk string retrieval**: Batch operations optimized for high-performance use cases
- **String interning**: Memory efficiency for duplicate strings using string-interner
- **MessagePack caching**: Compressed serialization for faster subsequent loads
- **Security validation**: Configurable limits to prevent memory exhaustion
- **Python API compatibility**: Drop-in replacement via PyO3 bindings
- **UTF-8 handling**: Graceful fallback for corrupted data
- **Parallel file loading**: Multi-threaded processing with Rayon

## Usage

### Rust API

```rust
use rust_tlk_parser::TLKParser;

let mut parser = TLKParser::new();
parser.parse_from_file("dialog.tlk")?;

// Single string lookup
if let Some(text) = parser.get_string(1234)? {
    println!("String 1234: {}", text);
}

// Batch lookup for performance (key optimization)
let str_refs = vec![100, 200, 300, 400];
let batch_result = parser.get_strings_batch(&str_refs)?;
println!("Retrieved {} strings in {:.2}ms", 
         batch_result.strings.len(), 
         batch_result.metrics.total_time_ms);
```

### Python API (Drop-in Replacement)

```python
from rust_tlk_parser import TLKParser

parser = TLKParser()
parser.read("dialog.tlk")

# Same API as original Python TLKParser
text = parser.get_string(1234)
strings = parser.get_all_strings(0, 100)
results = parser.search_strings("hello", case_sensitive=False)

# New high-performance batch method
batch_strings = parser.get_strings_batch([100, 200, 300, 400])
```

## Architecture

### TLK File Format
- **Header** (20 bytes): File type, version, language ID, string count, data offset
- **String Entries** (40 bytes each): Flags, sound ResRef, offset, size
- **String Data**: UTF-8 encoded strings referenced by entries

### Memory Layout
```
TLKParser {
    header: TLKHeader,
    entries: Vec<TLKStringEntry>,     // Loaded once
    string_data: Vec<u8>,             // All strings in memory
    string_cache: HashMap<usize, CachedString>,  // Interned strings
    interner: StringInterner,         // Deduplication
}
```

### Performance Optimizations
1. **Single file read**: Load entire TLK into memory once
2. **Pre-caching**: Intern first 100 strings immediately
3. **Batch operations**: Optimized bulk string retrieval
4. **String interning**: Reduce memory for duplicate strings
5. **Zero-copy access**: Direct indexing into string_data buffer

## Integration

The parser integrates into the NWN2 Editor at these points:

1. **ResourceManager** (`backend/parsers/resource_manager.py:24`):
   ```python
   # Replace:
   from .tlk import TLKParser
   # With:
   from rust_tlk_parser import TLKParser
   ```

2. **RuntimeDataClassGenerator** (`backend/gamedata/dynamic_loader/runtime_class_generator.py:172`):
   ```python
   # Use batch lookup instead of individual calls:
   batch_result = resource_manager.get_strings_batch(str_refs)
   ```

## Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Python bindings
cargo build --release --features python-bindings

# Run tests
cargo test

# Run benchmarks
cargo bench
```

## Testing

The parser includes comprehensive tests using real TLK fixtures:
- `backend/tests/fixtures/tlk/dialog_english.tlk`
- `backend/tests/fixtures/tlk/dialog_french.tlk`
- Various malformed and edge case files

## Security

Built-in security limits prevent resource exhaustion:
- Maximum file size (default: 100MB)
- Maximum string count (default: 1,000,000)
- Maximum individual string size (default: 64KB)

## Compatibility

The Python bindings provide 100% API compatibility with the original TLKParser:
- All existing methods work unchanged
- Same error handling behavior
- Compatible return types and semantics
- Additional performance methods for optimization

## Performance Metrics

Expected improvements for the NWN2 Editor startup scenario:
- **String lookup time**: 4.6s → <0.5s (10x faster)
- **Memory usage**: Similar or lower due to string interning
- **Cache performance**: MessagePack + compression for persistent speedup
- **Batch operations**: 100-1000x faster than individual lookups