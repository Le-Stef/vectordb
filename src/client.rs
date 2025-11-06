use crate::collection::Collection;
use crate::error::{Result, VectorDbError};
use crate::storage::Storage;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

struct CachedCollection {
    collection: Collection,
    last_access: u64,
}

pub struct VectorDbClient {
    storage: Storage,
    collections: Arc<RwLock<HashMap<String, CachedCollection>>>,
    max_cached: usize,
}

impl VectorDbClient {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let storage = Storage::new(path)?;
        let max_cached = std::env::var("VECTORDB_MAX_CACHED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20);

        Ok(Self {
            storage,
            collections: Arc::new(RwLock::new(HashMap::new())),
            max_cached,
        })
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn evict_lru(&self, colls: &mut HashMap<String, CachedCollection>) {
        if colls.len() < self.max_cached {
            return;
        }

        // trouver la collection la moins récemment utilisée
        if let Some((oldest_name, _)) = colls
            .iter()
            .min_by_key(|(_, cached)| cached.last_access)
        {
            let name = oldest_name.clone();
            colls.remove(&name);
        }
    }

    pub fn create_collection(&self, name: String, dimension: usize) -> Result<()> {
        let mut colls = self.collections.write().unwrap();

        if colls.contains_key(&name) || self.storage.collection_exists(&name) {
            return Err(VectorDbError::CollectionAlreadyExists(name));
        }

        let coll = Collection::new(name.clone(), dimension);
        self.storage.save_collection(&coll)?;

        self.evict_lru(&mut colls);
        colls.insert(name, CachedCollection {
            collection: coll,
            last_access: Self::now(),
        });

        Ok(())
    }

    pub fn create_collection_with_ivf(
        &self,
        name: String,
        dimension: usize,
        n_clusters: usize,
    ) -> Result<()> {
        let mut colls = self.collections.write().unwrap();

        if colls.contains_key(&name) || self.storage.collection_exists(&name) {
            return Err(VectorDbError::CollectionAlreadyExists(name));
        }

        let coll = Collection::new_with_ivf(name.clone(), dimension, n_clusters);
        self.storage.save_collection(&coll)?;

        self.evict_lru(&mut colls);
        colls.insert(name, CachedCollection {
            collection: coll,
            last_access: Self::now(),
        });

        Ok(())
    }

    pub fn get_collection(&self, name: &str) -> Result<()> {
        let mut collections = self.collections.write().unwrap();

        if !collections.contains_key(name) {
            let collection = self.storage.load_collection(name)?;
            self.evict_lru(&mut collections);
            collections.insert(name.to_string(), CachedCollection {
                collection,
                last_access: Self::now(),
            });
        } else {
            // update access time
            if let Some(cached) = collections.get_mut(name) {
                cached.last_access = Self::now();
            }
        }

        Ok(())
    }

    pub fn delete_collection(&self, name: &str) -> Result<()> {
        let mut collections = self.collections.write().unwrap();
        collections.remove(name);
        self.storage.delete_collection(name)?;
        Ok(())
    }

    pub fn list_collections(&self) -> Result<Vec<String>> {
        self.storage.list_collections()
    }

    pub fn with_collection<F, R>(&self, name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&Collection) -> R,
    {
        // try read lock first
        {
            let colls = self.collections.read().unwrap();
            if let Some(cached) = colls.get(name) {
                return Ok(f(&cached.collection));
            }
        }

        // not in cache, need to load with write lock
        let mut colls = self.collections.write().unwrap();

        // double-check in case another thread loaded it
        if !colls.contains_key(name) {
            let collection = self.storage.load_collection(name)?;
            self.evict_lru(&mut colls);
            colls.insert(name.to_string(), CachedCollection {
                collection,
                last_access: Self::now(),
            });
        }

        let cached = colls.get_mut(name).unwrap();
        cached.last_access = Self::now();
        Ok(f(&cached.collection))
    }

    pub fn with_collection_mut<F, R>(&self, name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut Collection) -> Result<R>,
    {
        let mut colls = self.collections.write().unwrap();

        // auto-load if not present
        if !colls.contains_key(name) {
            let collection = self.storage.load_collection(name)?;
            self.evict_lru(&mut colls);
            colls.insert(name.to_string(), CachedCollection {
                collection,
                last_access: Self::now(),
            });
        }

        let cached = colls
            .get_mut(name)
            .ok_or_else(|| VectorDbError::CollectionNotFound(name.to_string()))?;

        cached.last_access = Self::now();
        let res = f(&mut cached.collection)?;
        self.storage.save_collection(&cached.collection)?;
        Ok(res)
    }
}
