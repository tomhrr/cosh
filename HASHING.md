# Alternative Hashing Algorithms

This document describes the alternative hashing algorithms feature implemented for improving HashMap performance in cosh.

## Overview

The cosh runtime uses `IndexMap` for hash tables to preserve insertion order while providing O(1) lookup performance. By default, `IndexMap` uses the standard library's `RandomState` hasher (SipHash-based), which is cryptographically secure but not always the fastest for general-purpose use.

This implementation adds support for alternative, faster hashing algorithms that can improve performance for hash-intensive operations.

## Available Hashers

### Default (No Feature Flags)
- **Hasher**: Standard library `RandomState` (SipHash-based)
- **Characteristics**: Cryptographically secure, DoS-resistant, moderate performance
- **Use case**: Default behavior, maximum compatibility

### AHash (Feature: `ahash`)
- **Hasher**: `ahash::RandomState`
- **Characteristics**: Fast, DoS-resistant, good general-purpose performance
- **Use case**: Recommended for most performance-sensitive applications
- **Benchmark**: ~6-8% faster than default in typical workloads

### FNV (Feature: `fnv`)
- **Hasher**: `fnv::FnvBuildHasher`
- **Characteristics**: Very fast for small keys, not DoS-resistant
- **Use case**: Specialized use cases with trusted input data
- **Benchmark**: Similar to default, better for very small keys

## Usage

### Building with Alternative Hashers

```bash
# Build with AHash (recommended)
cargo build --features ahash

# Build with FNV
cargo build --features fnv

# Build with default hasher (no flags needed)
cargo build
```

### Performance Considerations

- **AHash**: Provides measurable performance improvements for typical hash operations
- **FNV**: Best for scenarios with many small string keys
- **Default**: Most compatible, adequate performance for most use cases

## Implementation Details

### Type Aliases
The implementation uses conditional compilation to select the appropriate hasher:

```rust
#[cfg(feature = "ahash")]
type ValueHashMap<K, V> = IndexMap<K, V, ahash::RandomState>;

#[cfg(feature = "fnv")]
type ValueHashMap<K, V> = IndexMap<K, V, fnv::FnvBuildHasher>;

#[cfg(not(any(feature = "ahash", feature = "fnv")))]
type ValueHashMap<K, V> = IndexMap<K, V>;
```

### Helper Functions
Helper functions create maps with the appropriate hasher:

```rust
#[cfg(feature = "ahash")]
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

This feature is fully backward compatible:
- Default behavior unchanged when no features are enabled
- Existing code continues to work without modification
- Serialization format remains consistent

## Security Considerations

- **AHash**: Maintains DoS resistance with better performance
- **FNV**: Not DoS-resistant, should only be used with trusted data
- **Default**: Maximum security, well-tested

## Future Work

- Consider adding other fast hashers like xxHash
- Profile real-world workloads to optimize hasher selection
- Add runtime hasher selection if needed