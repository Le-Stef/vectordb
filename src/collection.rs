use crate::distance::{cosine_distance, normalize_l2};
use crate::error::{Result, VectorDbError};
use crate::filter::{matches_filter, WhereFilter};
use crate::ivf::IVFIndex;
use crate::vector::{MetadataValue, VectorEntry};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    pub name: String,
    pub dimension: usize,
    pub use_ivf: bool,
    pub n_clusters: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Collection {
    pub config: CollectionConfig,
    vectors: HashMap<String, VectorEntry>,
    #[serde(skip)]
    ivf_index: Option<IVFIndex>,
    pub(crate) needs_rebuild: bool,
    #[serde(skip)]
    batch_mode: bool,
    modifications_count: usize,
    #[serde(skip)]
    last_query_time_ms: f64,
    #[serde(skip)]
    total_queries: usize,
}

impl Collection {
    pub fn new(name: String, dimension: usize) -> Self {
        Self {
            config: CollectionConfig {
                name,
                dimension,
                use_ivf: false,
                n_clusters: 0,
            },
            vectors: HashMap::new(),
            ivf_index: None,
            needs_rebuild: false,
            batch_mode: false,
            modifications_count: 0,
            last_query_time_ms: 0.0,
            total_queries: 0,
        }
    }

    pub fn new_with_ivf(name: String, dimension: usize, n_clusters: usize) -> Self {
        Self {
            config: CollectionConfig {
                name,
                dimension,
                use_ivf: true,
                n_clusters,
            },
            vectors: HashMap::new(),
            ivf_index: Some(IVFIndex::new(n_clusters)),
            needs_rebuild: true,
            batch_mode: false,
            modifications_count: 0,
            last_query_time_ms: 0.0,
            total_queries: 0,
        }
    }

    pub fn begin_batch(&mut self) {
        self.batch_mode = true;
    }

    pub fn end_batch(&mut self) {
        self.batch_mode = false;
        if self.config.use_ivf && self.modifications_count > 0 {
            self.needs_rebuild = true;
        }
    }

    pub fn add(
        &mut self,
        ids: Vec<String>,
        embeddings: Vec<Vec<f32>>,
        metadatas: Option<Vec<HashMap<String, MetadataValue>>>,
    ) -> Result<()> {
        let n = ids.len();
        if n != embeddings.len() {
            return Err(VectorDbError::InvalidConfig(
                "ids and embeddings must have the same length".to_string(),
            ));
        }

        if let Some(ref metas) = metadatas {
            if metas.len() != n {
                return Err(VectorDbError::InvalidConfig(
                    "metadatas must have the same length as ids".to_string(),
                ));
            }
        }

        // pre-reserve capacity si nécessaire
        if self.vectors.capacity() < self.vectors.len() + n {
            self.vectors.reserve(n);
        }

        for idx in 0..n {
            let mut embedding = embeddings[idx].clone();

            if embedding.len() != self.config.dimension {
                return Err(VectorDbError::DimensionMismatch {
                    expected: self.config.dimension,
                    actual: embedding.len(),
                });
            }

            normalize_l2(&mut embedding);

            let metadata = metadatas
                .as_ref()
                .and_then(|m| m.get(idx))
                .cloned()
                .unwrap_or_default();

            let entry = VectorEntry {
                id: ids[idx].clone(),
                embedding,
                metadata,
            };
            self.vectors.insert(ids[idx].clone(), entry);
        }

        // marquer qu'on doit rebuild l'IVF (sauf en batch mode)
        if self.config.use_ivf {
            self.modifications_count += n;
            if !self.batch_mode {
                self.needs_rebuild = true;
            }
        }

        Ok(())
    }

    pub fn get(
        &self,
        ids: Option<Vec<String>>,
        include: Option<Vec<String>>,
    ) -> Result<GetResult> {
        use std::collections::HashSet;

        let default_include = vec!["metadatas".to_string(), "embeddings".to_string()];
        let include_set: HashSet<String> = include
            .unwrap_or(default_include)
            .into_iter()
            .collect();

        let entries: Vec<&VectorEntry> = match ids {
            Some(id_list) => id_list
                .iter()
                .filter_map(|id| self.vectors.get(id))
                .collect(),
            None => self.vectors.values().collect(),
        };

        let result_ids = entries.iter().map(|e| e.id.clone()).collect();

        let embeddings = if include_set.contains("embeddings") {
            Some(entries.iter().map(|e| e.embedding.clone()).collect())
        } else {
            None
        };

        let metadatas = if include_set.contains("metadatas") {
            Some(entries.iter().map(|e| e.metadata.clone()).collect())
        } else {
            None
        };

        Ok(GetResult {
            ids: result_ids,
            embeddings,
            metadatas,
        })
    }

    pub fn update(
        &mut self,
        ids: Vec<String>,
        metadatas: Vec<HashMap<String, MetadataValue>>,
    ) -> Result<()> {
        if ids.len() != metadatas.len() {
            return Err(VectorDbError::InvalidConfig(
                "ids and metadatas must have the same length".to_string(),
            ));
        }

        for (idx, id) in ids.iter().enumerate() {
            let entry = self.vectors
                .get_mut(id)
                .ok_or_else(|| VectorDbError::VectorNotFound(id.clone()))?;

            // merge metadata au lieu de remplacer
            for (k, v) in &metadatas[idx] {
                entry.metadata.insert(k.clone(), v.clone());
            }
        }

        Ok(())
    }

    pub fn delete(&mut self, ids: Vec<String>) -> Result<()> {
        let n = ids.len();
        ids.iter().for_each(|id| {
            self.vectors.remove(id);
        });

        if self.config.use_ivf {
            self.modifications_count += n;
            if !self.batch_mode {
                self.needs_rebuild = true;
            }
        }

        Ok(())
    }

    pub fn count(&self) -> usize {
        self.vectors.len()
    }

    pub fn stats(&self) -> CollectionStats {
        let index_info = if self.config.use_ivf {
            if let Some(ref ivf) = self.ivf_index {
                Some(IndexInfo {
                    is_built: ivf.is_built(),
                    n_clusters: self.config.n_clusters,
                    n_centroids: ivf.centroids.len(),
                    needs_rebuild: self.needs_rebuild,
                })
            } else {
                None
            }
        } else {
            None
        };

        // estimation mémoire approximative
        let vec_size = self.vectors.len() * (self.config.dimension * 4 + 64); // f32 + overhead
        let index_size = if let Some(ref ivf) = self.ivf_index {
            ivf.centroids.len() * self.config.dimension * 4
        } else {
            0
        };

        CollectionStats {
            name: self.config.name.clone(),
            dimension: self.config.dimension,
            count: self.vectors.len(),
            use_ivf: self.config.use_ivf,
            index_info,
            estimated_memory_bytes: vec_size + index_size,
            last_query_time_ms: self.last_query_time_ms,
            total_queries: self.total_queries,
        }
    }

    // rebuilder l'index IVF si nécessaire
    pub fn rebuild_index(&mut self) {
        if !self.config.use_ivf || !self.needs_rebuild {
            return;
        }

        if let Some(ref mut ivf) = self.ivf_index {
            let data: Vec<(String, Vec<f32>)> = self.vectors.iter()
                .map(|(id, v)| (id.clone(), v.embedding.clone()))
                .collect();

            if !data.is_empty() {
                ivf.rebuild(&data);
                self.needs_rebuild = false;
                self.modifications_count = 0;
            }
        }
    }

    // rebuild automatique si trop de modifications (seuil : 10%)
    fn maybe_rebuild(&mut self) {
        if !self.config.use_ivf || !self.needs_rebuild {
            return;
        }

        let total = self.vectors.len();
        if total == 0 {
            return;
        }

        // rebuild si plus de 10% de modifications
        let threshold = (total as f64 * 0.1).max(10.0) as usize;
        if self.modifications_count >= threshold {
            self.rebuild_index();
        }
    }

    pub fn query(
        &mut self,
        query_embedding: &[f32],
        n_results: usize,
        where_filter: Option<&WhereFilter>,
    ) -> Result<Vec<SearchResult>> {
        use std::time::Instant;

        let start = Instant::now();

        if query_embedding.len() != self.config.dimension {
            return Err(VectorDbError::DimensionMismatch {
                expected: self.config.dimension,
                actual: query_embedding.len(),
            });
        }

        self.maybe_rebuild();

        let mut normalized_query = query_embedding.to_vec();
        normalize_l2(&mut normalized_query);

        let mut results = if self.config.use_ivf {
            if let Some(ref ivf) = self.ivf_index {
                if ivf.is_built() {
                    self.query_with_ivf(&normalized_query, n_results, where_filter)?
                } else {
                    self.query_linear(&normalized_query, n_results, where_filter)?
                }
            } else {
                self.query_linear(&normalized_query, n_results, where_filter)?
            }
        } else {
            self.query_linear(&normalized_query, n_results, where_filter)?
        };

        // appliquer filtre si présent
        if let Some(filter) = where_filter {
            results.retain(|r| matches_filter(&r.metadata, filter));
            results.truncate(n_results);
        }

        self.last_query_time_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.total_queries += 1;

        Ok(results)
    }

    fn query_linear(&self, normalized_query: &[f32], n_results: usize, where_filter: Option<&WhereFilter>) -> Result<Vec<SearchResult>> {
        // filtrer d'abord si nécessaire
        let entries_to_search: Vec<&VectorEntry> = if let Some(filter) = where_filter {
            self.vectors.values()
                .filter(|entry| matches_filter(&entry.metadata, filter))
                .collect()
        } else {
            self.vectors.values().collect()
        };

        // paralléliser si suffisamment de vecteurs
        let mut results: Vec<SearchResult> = if entries_to_search.len() > 100 {
            entries_to_search.par_iter()
                .map(|entry| {
                    let dist = cosine_distance(normalized_query, &entry.embedding);
                    SearchResult {
                        id: entry.id.clone(),
                        distance: dist,
                        metadata: entry.metadata.clone(),
                    }
                })
                .collect()
        } else {
            entries_to_search.iter()
                .map(|entry| {
                    let dist = cosine_distance(normalized_query, &entry.embedding);
                    SearchResult {
                        id: entry.id.clone(),
                        distance: dist,
                        metadata: entry.metadata.clone(),
                    }
                })
                .collect()
        };

        // tri partiel suffit pour n_results << total
        if n_results < results.len() / 4 {
            results.select_nth_unstable_by(n_results, |a, b| {
                a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal)
            });
            results.truncate(n_results);
            results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        } else {
            results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
            results.truncate(n_results);
        }

        Ok(results)
    }

    fn query_with_ivf(&self, normalized_query: &[f32], n_results: usize, where_filter: Option<&WhereFilter>) -> Result<Vec<SearchResult>> {
        let ivf = self.ivf_index.as_ref().unwrap();
        let candidate_ids = ivf.search_candidates(normalized_query);

        // paralléliser le calcul des distances sur les candidats
        let mut results: Vec<SearchResult> = if candidate_ids.len() > 50 {
            candidate_ids.par_iter()
                .filter_map(|id| self.vectors.get(id))
                .filter(|entry| {
                    where_filter.map_or(true, |f| matches_filter(&entry.metadata, f))
                })
                .map(|entry| {
                    let dist = cosine_distance(normalized_query, &entry.embedding);
                    SearchResult {
                        id: entry.id.clone(),
                        distance: dist,
                        metadata: entry.metadata.clone(),
                    }
                })
                .collect()
        } else {
            candidate_ids.iter()
                .filter_map(|id| self.vectors.get(id))
                .filter(|entry| {
                    where_filter.map_or(true, |f| matches_filter(&entry.metadata, f))
                })
                .map(|entry| {
                    let dist = cosine_distance(normalized_query, &entry.embedding);
                    SearchResult {
                        id: entry.id.clone(),
                        distance: dist,
                        metadata: entry.metadata.clone(),
                    }
                })
                .collect()
        };

        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        results.truncate(n_results);

        Ok(results)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetResult {
    pub ids: Vec<String>,
    pub embeddings: Option<Vec<Vec<f32>>>,
    pub metadatas: Option<Vec<HashMap<String, MetadataValue>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub distance: f32,
    pub metadata: HashMap<String, MetadataValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexInfo {
    pub is_built: bool,
    pub n_clusters: usize,
    pub n_centroids: usize,
    pub needs_rebuild: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionStats {
    pub name: String,
    pub dimension: usize,
    pub count: usize,
    pub use_ivf: bool,
    pub index_info: Option<IndexInfo>,
    pub estimated_memory_bytes: usize,
    pub last_query_time_ms: f64,
    pub total_queries: usize,
}
