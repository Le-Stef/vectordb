use crate::collection::Collection;
use crate::error::{Result, VectorDbError};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

pub struct Storage {
    base_path: PathBuf,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        fs::create_dir_all(&base_path)?;
        fs::create_dir_all(base_path.join("collections"))?;

        Ok(Self { base_path })
    }

    pub fn collection_path(&self, name: &str) -> PathBuf {
        self.base_path.join("collections").join(name)
    }

    pub fn save_collection(&self, collection: &Collection) -> Result<()> {
        let coll_path = self.collection_path(&collection.config.name);
        fs::create_dir_all(&coll_path)?;

        // sauvegarder en bincode pour meilleure perf
        let data_path = coll_path.join("data.bin");
        let f = File::create(data_path)?;
        let writer = BufWriter::with_capacity(512 * 1024, f);
        bincode::serialize_into(writer, collection)?;

        Ok(())
    }

    pub fn load_collection(&self, name: &str) -> Result<Collection> {
        let coll_path = self.collection_path(name);

        // essayer bincode d'abord (nouveau format)
        let bin_path = coll_path.join("data.bin");
        if bin_path.exists() {
            let file = File::open(bin_path)?;
            let reader = BufReader::with_capacity(512 * 1024, file);
            let mut collection: Collection = bincode::deserialize_from(reader)?;
            // reconstruire l'index IVF si nécessaire
            if collection.config.use_ivf {
                collection.needs_rebuild = true;
            }
            return Ok(collection);
        }

        // fallback sur JSON (ancien format)
        let json_path = coll_path.join("data.json");
        if json_path.exists() {
            let file = File::open(json_path)?;
            let reader = BufReader::new(file);
            let mut collection: Collection = serde_json::from_reader(reader)?;
            if collection.config.use_ivf {
                collection.needs_rebuild = true;
            }
            return Ok(collection);
        }

        Err(VectorDbError::CollectionNotFound(name.to_string()))
    }

    pub fn delete_collection(&self, name: &str) -> Result<()> {
        let coll_path = self.collection_path(name);
        if coll_path.exists() {
            fs::remove_dir_all(coll_path)?;
        }
        Ok(())
    }

    pub fn list_collections(&self) -> Result<Vec<String>> {
        let coll_dir = self.base_path.join("collections");
        if !coll_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(coll_dir)?;
        let mut names = Vec::new();

        for entry in entries {
            if let Ok(e) = entry {
                if e.path().is_dir() {
                    if let Some(name) = e.file_name().to_str() {
                        names.push(name.to_string());
                    }
                }
            }
        }

        Ok(names)
    }

    pub fn collection_exists(&self, name: &str) -> bool {
        let path = self.collection_path(name);
        path.join("data.bin").exists() || path.join("data.json").exists()
    }
}
