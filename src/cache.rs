use crate::DishSizeIngredients;
use eyre::Context;
use lru::LruCache;
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{LazyLock, Mutex, Once},
};

pub struct IngredientsCache {
    cache: LazyLock<Mutex<LruCache<i64, DishSizeIngredients>>>,
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

    fn get_cache_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        path.push("powermeal-ai");
        path.push("ingredients_cache.json");
        path
    }

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
    
    pub fn get(&self, id: &i64) -> Option<DishSizeIngredients> {
        let mut cache_guard = self.cache.lock().unwrap();
        cache_guard.get(id).cloned()
    }
    
    pub fn put(&self, id: i64, ingredients: DishSizeIngredients) {
        let mut cache_guard = self.cache.lock().unwrap();
        cache_guard.put(id, ingredients);
    }
}