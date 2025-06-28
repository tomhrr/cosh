use indexmap::IndexMap;
use crate::chunk::Value;

// Use compile-time features to select hasher
// Default to AHash for performance, but allow override

#[cfg(feature = "fxhash")]
use fxhash::FxBuildHasher;
#[cfg(feature = "fxhash")]
pub type CoshIndexMap<K, V> = IndexMap<K, V, FxBuildHasher>;

#[cfg(all(feature = "default-hasher", not(feature = "fxhash")))]
pub type CoshIndexMap<K, V> = IndexMap<K, V>;

#[cfg(all(not(feature = "default-hasher"), not(feature = "fxhash")))]
use ahash::RandomState as AHashRandomState;
#[cfg(all(not(feature = "default-hasher"), not(feature = "fxhash")))]
pub type CoshIndexMap<K, V> = IndexMap<K, V, AHashRandomState>;

/// Create a new IndexMap for hash values using the configured hasher.
pub fn new_hash_indexmap() -> CoshIndexMap<String, Value> {
    #[cfg(feature = "fxhash")]
    {
        IndexMap::with_hasher(FxBuildHasher::default())
    }
    #[cfg(all(feature = "default-hasher", not(feature = "fxhash")))]
    {
        IndexMap::new()
    }
    #[cfg(all(not(feature = "default-hasher"), not(feature = "fxhash")))]
    {
        IndexMap::with_hasher(AHashRandomState::new())
    }
}

/// Create a new IndexMap for set values using the configured hasher.
pub fn new_set_indexmap() -> CoshIndexMap<String, Value> {
    new_hash_indexmap()
}

/// Create a generic IndexMap with the configured hasher.
pub fn new_indexmap<K, V>() -> CoshIndexMap<K, V> {
    #[cfg(feature = "fxhash")]
    {
        IndexMap::with_hasher(FxBuildHasher::default())
    }
    #[cfg(all(feature = "default-hasher", not(feature = "fxhash")))]
    {
        IndexMap::new()
    }
    #[cfg(all(not(feature = "default-hasher"), not(feature = "fxhash")))]
    {
        IndexMap::with_hasher(AHashRandomState::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_set_creation() {
        // Test that helper functions work
        let _hash_map = new_hash_indexmap();
        let _set_map = new_set_indexmap();
    }

    #[test]
    fn test_indexmap_functionality() {
        // Test that the created IndexMaps work correctly
        let mut map = new_hash_indexmap();
        map.insert("test".to_string(), Value::Null);
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("test"));
    }

    #[test]
    fn test_generic_indexmap() {
        let mut map: CoshIndexMap<String, i32> = new_indexmap();
        map.insert("key".to_string(), 42);
        assert_eq!(map.get("key"), Some(&42));
    }

    #[test]
    fn test_indexmap_ordering() {
        // Test that IndexMap maintains insertion order with custom hashers
        let mut map = new_hash_indexmap();
        map.insert("first".to_string(), Value::Int(1));
        map.insert("second".to_string(), Value::Int(2));
        map.insert("third".to_string(), Value::Int(3));

        let keys: Vec<_> = map.keys().collect();
        assert_eq!(keys, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_hash_vs_set_maps() {
        // Ensure both helper functions work and are compatible
        let mut hash_map = new_hash_indexmap();
        let mut set_map = new_set_indexmap();
        
        hash_map.insert("key1".to_string(), Value::Int(100));
        set_map.insert("key2".to_string(), Value::Int(200));
        
        assert_eq!(hash_map.len(), 1);
        assert_eq!(set_map.len(), 1);
        assert!(hash_map.contains_key("key1"));
        assert!(set_map.contains_key("key2"));
    }

    #[test]
    fn test_type_compatibility() {
        // Test that CoshIndexMap is compatible with Value enum
        let mut map = new_hash_indexmap();
        
        // Test different Value types
        map.insert("null".to_string(), Value::Null);
        map.insert("bool".to_string(), Value::Bool(true));
        map.insert("int".to_string(), Value::Int(42));
        map.insert("float".to_string(), Value::Float(3.14));
        
        assert_eq!(map.len(), 4);
        assert!(map.contains_key("bool"));
        assert!(map.contains_key("int"));
        assert!(map.contains_key("null"));
        assert!(map.contains_key("float"));
    }
}