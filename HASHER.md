# Alternative Hashing Algorithms for Cosh

This implementation adds support for alternative hashing algorithms to improve HashMap performance in the cosh language.

## Overview

The cosh language uses `IndexMap` from the `indexmap` crate for its Hash and Set data types. By default, `IndexMap` uses the standard library's default hasher, which prioritizes cryptographic security over performance. For many use cases, faster non-cryptographic hashers can provide significant performance improvements.

## Supported Hashers

### AHash (Default)
- **Algorithm**: AHash  
- **Performance**: Fastest, high-quality hashing
- **Usage**: Default when no features are specified
- **Best for**: General purpose, maximum performance

### FxHash
- **Algorithm**: FxHash (Firefox's hasher)
- **Performance**: Simple, fast hashing  
- **Usage**: `cargo build --features fxhash`
- **Best for**: Simple keys, embedded systems

### Standard Library Hasher
- **Algorithm**: DefaultHasher (SipHash)
- **Performance**: Slower, cryptographically secure
- **Usage**: `cargo build --features default-hasher`  
- **Best for**: Security-sensitive applications

## Usage

### Build Commands

```bash
# Default build (AHash - recommended)
make

# Use FxHash
cargo build --release --features fxhash

# Use standard library hasher
cargo build --release --features default-hasher
```

### Testing Different Hashers

```bash
# Test with default AHash
make test

# Test with FxHash
cargo test --release --features fxhash

# Test with standard hasher
cargo test --release --features default-hasher
```

## Implementation Details

### Design Principles

1. **Zero Runtime Overhead**: Hasher selection happens at compile time
2. **API Compatibility**: No changes to existing cosh language semantics
3. **Type Safety**: All IndexMap instances use the same hasher type
4. **Centralized Control**: Single module manages all hasher configuration

### Technical Approach

The implementation uses Rust's feature system to select hashers at compile time:

```rust
// Type alias resolves to different concrete types based on features
pub type CoshIndexMap<K, V> = IndexMap<K, V, SelectedHasher>;

// Helper functions create IndexMaps with the correct hasher
pub fn new_hash_indexmap() -> CoshIndexMap<String, Value> { ... }
pub fn new_set_indexmap() -> CoshIndexMap<String, Value> { ... }
```

### Files Modified

- `src/hasher.rs` - New hasher configuration module
- `src/chunk.rs` - Updated Value enum to use CoshIndexMap
- All VM modules - Updated IndexMap creation sites

## Performance Considerations

### Expected Improvements

- **AHash**: 2-4x faster than SipHash for most workloads
- **FxHash**: 2-3x faster than SipHash for simple keys
- **Memory**: No additional memory overhead

### Benchmarking

The implementation maintains insertion order (IndexMap property) while improving hash performance. For hash-intensive workloads like JSON processing, environment variable access, and data structure manipulation, performance improvements should be noticeable.

## Security Considerations

### Hash Collision Attacks

- **AHash**: Resistant to hash flooding attacks
- **FxHash**: Vulnerable to deliberate collision attacks (use only with trusted input)
- **SipHash**: Cryptographically secure against all collision attacks

### Recommendations

- **Default AHash**: Safe for most applications
- **FxHash**: Only use when input is trusted and performance is critical
- **SipHash**: Use when processing untrusted input in security-sensitive contexts

## Migration Guide

This implementation is fully backward compatible. Existing cosh code will continue to work without changes. To take advantage of alternative hashers:

1. Rebuild with desired feature flags
2. No code changes required
3. Hash iteration order remains consistent (IndexMap property preserved)

## Future Enhancements

Potential future improvements:

1. Runtime hasher selection via environment variables
2. Per-data-structure hasher configuration
3. Custom hasher implementations for domain-specific use cases
4. Performance benchmarking suite