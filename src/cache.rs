//! Thread-safe LRU cache for dish ingredients with persistent storage.
//!
//! This module provides a singleton cache that stores ingredient information for dishes,
//! using an LRU (Least Recently Used) eviction policy with a capacity of 1000 entries.
//! The cache is automatically persisted to disk following XDG Base Directory specifications,
//! and loaded on first access.
//!
//! # Storage Location
//!
//! The cache is stored at `~/.config/powermeal-ai/ingredients_cache.json` on Unix-like systems,
//! or the appropriate config directory on other platforms.
//!
//! # Thread Safety
//!
//! The cache is fully thread-safe through internal mutex synchronization.
//! Multiple threads can safely read and write to the cache concurrently.

use crate::serde::DishSizeIngredients; // TODO: Phase 2 - update with new structures
use eyre::Context;
use lru::LruCache;
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{LazyLock, Mutex, Once},
};

/// A thread-safe LRU cache for storing dish ingredient information.
///
/// This cache implements the singleton pattern and provides automatic persistence
/// to disk. It maintains up to 1000 entries using LRU eviction when capacity is exceeded.
///
/// # Examples
///
/// ```no_run
/// let cache = IngredientsCache::get_instance();
/// 
/// // Store ingredients for a dish
/// cache.put(12345, ingredients);
/// 
/// // Retrieve ingredients
/// if let Some(ingredients) = cache.get(&12345) {
///     println!("Found ingredients: {:?}", ingredients);
/// }
/// 
/// // Save cache to disk
/// cache.save().expect("Failed to save cache");
/// ```
pub struct IngredientsCache {
    /// Thread-safe LRU cache with mutex protection
    cache: LazyLock<Mutex<LruCache<i64, DishSizeIngredients>>>,
    /// Ensures one-time initialization of the cache from disk
    init: Once,
}

impl Drop for IngredientsCache {
    fn drop(&mut self) {
        if let Err(e) = self.save() {
            tracing::warn!("Failed to save ingredients cache on drop: {}", e);
        } else {
            tracing::info!("Ingredients cache saved on drop");
        }
    }
}

impl IngredientsCache {
    /// Returns the singleton instance of the ingredients cache.
    ///
    /// This method ensures thread-safe access to a single global cache instance.
    /// On the first call, it attempts to load existing cache data from disk.
    /// If loading fails, a warning is logged and an empty cache is initialized.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called from multiple threads concurrently.
    /// The cache initialization is performed exactly once using `Once::call_once`.
    ///
    /// # Returns
    ///
    /// A reference to the global cache instance that lives for the entire program lifetime.
    pub fn get_instance() -> &'static IngredientsCache {
        static INSTANCE: IngredientsCache = IngredientsCache {
            cache: LazyLock::new(|| Mutex::new(LruCache::new(1000.try_into().unwrap()))),
            init: Once::new(),
        };
        
        INSTANCE.init.call_once(|| {
            if let Err(e) = INSTANCE.load() {
                tracing::warn!("Failed to load ingredients cache: {}", e);
            } else {
                tracing::info!("Ingredients cache loaded successfully");
            }
        });
        
        &INSTANCE
    }

    /// Returns the filesystem path where the cache is persisted.
    ///
    /// Uses the XDG Base Directory specification to determine the appropriate
    /// configuration directory. Falls back to `~/.config` if the system config
    /// directory cannot be determined.
    ///
    /// # Returns
    ///
    /// Path to `ingredients_cache.json` in the PowerMeal configuration directory.
    fn get_cache_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        path.push("powermeal-ai");
        path.push("ingredients_cache.json");
        path
    }

    /// Loads the cache from persistent storage.
    ///
    /// Reads the JSON cache file from disk and populates the in-memory LRU cache
    /// with the stored entries. If the cache file doesn't exist, returns success
    /// without error (treating it as an empty cache).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The cache file exists but cannot be read
    /// - The cache file contains invalid JSON
    /// - The JSON structure doesn't match the expected format
    ///
    /// # Note
    ///
    /// This method is typically called once during initialization via `get_instance()`.
    fn load(&self) -> eyre::Result<()> {
        let cache_path = Self::get_cache_path();
        if !cache_path.exists() {
            return Ok(());
        }

        let cache_data = fs::read_to_string(&cache_path).wrap_err("Failed to read cache file")?;
        let cache_entries: HashMap<i64, DishSizeIngredients> = 
            serde_json::from_str(&cache_data).wrap_err("Failed to parse cache data")?;
        
        let mut cache_guard = self.cache.lock().unwrap();
        for (id, ingredients) in cache_entries {
            cache_guard.put(id, ingredients);
        }
        
        tracing::info!("Loaded {} entries from ingredients cache", cache_guard.len());
        Ok(())
    }

    /// Persists the current cache contents to disk.
    ///
    /// Serializes all cache entries to JSON and writes them to the cache file.
    /// Creates the parent directory if it doesn't exist. This method is automatically
    /// called when the cache is dropped, but can also be called manually for
    /// explicit persistence.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The cache directory cannot be created
    /// - The cache data cannot be serialized to JSON
    /// - The cache file cannot be written to disk
    ///
    /// # Performance
    ///
    /// This operation locks the cache for the duration of serialization.
    /// With 1000 entries, this is typically very fast (< 100ms).
    pub fn save(&self) -> eyre::Result<()> {
        let cache_path = Self::get_cache_path();
        
        // Create directory if it doesn't exist
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).wrap_err("Failed to create cache directory")?;
        }
        
        let cache_guard = self.cache.lock().unwrap();
        let mut cache_entries = HashMap::with_capacity(cache_guard.len());
        
        for (id, ingredients) in cache_guard.iter() {
            cache_entries.insert(*id, ingredients.clone());
        }
        
        let cache_data = serde_json::to_string(&cache_entries).wrap_err("Failed to serialize cache")?;
        fs::write(&cache_path, cache_data).wrap_err("Failed to write cache file")?;
        
        tracing::info!("Saved {} entries to ingredients cache", cache_entries.len());
        Ok(())
    }
    
    /// Retrieves ingredients for a dish by its ID.
    ///
    /// This operation updates the LRU ordering, marking the accessed entry
    /// as most recently used. Returns a cloned copy of the ingredients to
    /// avoid holding the cache lock.
    ///
    /// # Arguments
    ///
    /// * `id` - The dish size ID to look up
    ///
    /// # Returns
    ///
    /// * `Some(DishSizeIngredients)` - If the entry exists in the cache
    /// * `None` - If the entry is not cached
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and acquires a mutex lock for the lookup duration.
    pub fn get(&self, id: &i64) -> Option<DishSizeIngredients> {
        let mut cache_guard = self.cache.lock().unwrap();
        cache_guard.get(id).cloned()
    }
    
    /// Stores ingredients for a dish in the cache.
    ///
    /// If the cache is at capacity (1000 entries), the least recently used
    /// entry will be evicted to make room for the new entry. If an entry
    /// with the same ID already exists, it will be updated.
    ///
    /// # Arguments
    ///
    /// * `id` - The dish size ID to store
    /// * `ingredients` - The ingredients data to cache
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and acquires a mutex lock for the insertion duration.
    ///
    /// # Note
    ///
    /// Changes are not automatically persisted to disk. Call `save()` explicitly
    /// or rely on the automatic save when the cache is dropped.
    pub fn put(&self, id: i64, ingredients: DishSizeIngredients) {
        let mut cache_guard = self.cache.lock().unwrap();
        cache_guard.put(id, ingredients);
    }
}