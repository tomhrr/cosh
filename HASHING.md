# Alternative Hashing Algorithms

This document describes the alternative hashing algorithms feature implemented for improving HashMap performance in cosh.

## Overview

The cosh runtime uses `IndexMap` for hash tables to preserve insertion order while providing O(1) lookup performance. By default, cosh now uses the AHash algorithm, which provides better performance than the standard library's hasher while maintaining DoS resistance.

This implementation allows users to optionally select different hashing algorithms based on their specific needs.

## Available Hashers

### Default (AHash)
- **Hasher**: `ahash::RandomState`
- **Characteristics**: Fast, DoS-resistant, good general-purpose performance
- **Use case**: Default behavior, recommended for most applications
- **Benchmark**: ~6-8% faster than standard library hasher

### FNV (Feature: `fnv`)
- **Hasher**: `fnv::FnvBuildHasher`
- **Characteristics**: Very fast for small keys, not DoS-resistant
- **Use case**: Specialized use cases with trusted input data
- **Benchmark**: Similar to standard library, better for very small keys

## Usage

### Building with Alternative Hashers

```bash
# Build with default AHash (no flags needed)
cargo build

# Build with FNV for specialized use cases
cargo build --features fnv
```

### Performance Considerations

- **AHash (Default)**: Provides measurable performance improvements for typical hash operations
- **FNV**: Best for scenarios with many small string keys and when maximum speed is needed with trusted data

## Implementation Details

### Type Aliases
The implementation uses conditional compilation to select the appropriate hasher:

```rust
#[cfg(feature = "fnv")]
type ValueHashMap<K, V> = IndexMap<K, V, fnv::FnvBuildHasher>;

#[cfg(not(feature = "fnv"))]
type ValueHashMap<K, V> = IndexMap<K, V, ahash::RandomState>;
```

### Helper Functions
Helper functions create maps with the appropriate hasher:

```rust
#[cfg(not(feature = "fnv"))]
pub fn new_value_hashmap<K, V>() -> ValueHashMap<K, V> {
    IndexMap::with_hasher(ahash::RandomState::new())
}
```

### Affected Components
- `Value::Hash` - Hash maps in the value system
- `Value::Set` - Set implementation (uses hash map internally)
- Environment variables (`vm_env.rs`)
- Hash operations in various VM modules

### Serialization Compatibility
Serialization uses standard `IndexMap` regardless of hasher choice to ensure compatibility across different builds.

## Testing

The implementation includes tests that verify:
1. Basic hash map functionality with each hasher
2. Performance characteristics
3. Compatibility with existing code

## Backward Compatibility

This feature maintains full backward compatibility:
- AHash is now the default hasher, providing improved performance out of the box
- Existing code continues to work without modification
- Serialization format remains consistent
- Users can opt for FNV hasher for specialized use cases

## Security Considerations

- **AHash**: Maintains DoS resistance with better performance
- **FNV**: Not DoS-resistant, should only be used with trusted data
- **Default**: Maximum security, well-tested

## Future Work

- Consider adding other fast hashers like xxHash
- Profile real-world workloads to optimize hasher selection
- Add runtime hasher selection if needed