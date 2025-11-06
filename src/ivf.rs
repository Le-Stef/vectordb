use crate::distance::cosine_distance;
use crate::kmeans::KMeans;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IVFIndex {
    pub centroids: Vec<Vec<f32>>,
    pub inverted_lists: Vec<Vec<String>>,  // stocke les IDs directement
    pub n_clusters: usize,
    pub n_probe: usize,
}

impl IVFIndex {
    pub fn new(n_clusters: usize) -> Self {
        Self {
            centroids: Vec::new(),
            inverted_lists: vec![Vec::new(); n_clusters],
            n_clusters,
            n_probe: 4,  // valeur par défaut, chercher dans 4 clusters les plus proches
        }
    }

    pub fn with_n_probe(mut self, n_probe: usize) -> Self {
        self.n_probe = n_probe.min(self.n_clusters);
        self
    }

    // construire l'index à partir des vecteurs avec leurs IDs
    pub fn build(&mut self, data: &[(String, Vec<f32>)]) {
        if data.is_empty() {
            return;
        }

        let embeddings: Vec<Vec<f32>> = data.iter().map(|(_, emb)| emb.clone()).collect();

        // réduire n_clusters si pas assez de vecteurs
        let actual_clusters = self.n_clusters.min(embeddings.len() / 10).max(1);

        let mut kmeans = KMeans::new(actual_clusters);
        kmeans.fit(&embeddings);

        self.centroids = kmeans.centroids.clone();
        self.inverted_lists = vec![Vec::new(); actual_clusters];

        // assigner chaque vecteur à son cluster
        for (id, emb) in data.iter() {
            let cluster = kmeans.predict(emb);
            self.inverted_lists[cluster].push(id.clone());
        }
    }

    // chercher les n_probe clusters les plus proches du query
    pub fn search_candidates(&self, query: &[f32]) -> Vec<String> {
        if self.centroids.is_empty() {
            return Vec::new();
        }

        let mut distances: Vec<(usize, f32)> = self.centroids.iter()
            .enumerate()
            .map(|(idx, c)| (idx, cosine_distance(query, c)))
            .collect();

        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let probe_count = self.n_probe.min(distances.len());
        let mut candidates = Vec::new();

        for i in 0..probe_count {
            let cluster_idx = distances[i].0;
            candidates.extend(self.inverted_lists[cluster_idx].iter().cloned());
        }

        candidates
    }

    // rebuild après ajout/suppression de vecteurs
    pub fn rebuild(&mut self, data: &[(String, Vec<f32>)]) {
        self.build(data);
    }

    pub fn is_built(&self) -> bool {
        !self.centroids.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ivf_build() {
        // créer plus de données pour éviter la réduction auto de n_clusters
        let mut data = vec![];
        for i in 0..100 {
            let x = (i as f32 / 33.0) % 1.0;
            let y = (i as f32 / 11.0) % 1.0;
            let z = 1.0 - x - y;
            data.push((format!("id{}", i), vec![x, y, z.max(0.0)]));
        }

        let mut ivf = IVFIndex::new(5);
        ivf.build(&data);

        assert!(ivf.centroids.len() >= 3);
        assert!(ivf.is_built());
    }

    #[test]
    fn test_ivf_search() {
        let data = vec![
            ("id1".to_string(), vec![1.0, 0.0, 0.0]),
            ("id2".to_string(), vec![0.9, 0.1, 0.0]),
            ("id3".to_string(), vec![0.0, 1.0, 0.0]),
            ("id4".to_string(), vec![0.0, 0.9, 0.1]),
        ];

        let mut ivf = IVFIndex::new(2).with_n_probe(1);
        ivf.build(&data);

        let query = vec![0.95, 0.05, 0.0];
        let candidates = ivf.search_candidates(&query);

        assert!(!candidates.is_empty());
    }
}
