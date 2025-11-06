use crate::distance::cosine_distance;
use rand::{Rng, seq::SliceRandom};
use rayon::prelude::*;

#[derive(Debug, Clone)]
pub struct KMeans {
    pub centroids: Vec<Vec<f32>>,
    pub n_clusters: usize,
    pub max_iter: usize,
    pub tolerance: f32,
}

impl KMeans {
    pub fn new(n_clusters: usize) -> Self {
        Self {
            centroids: Vec::new(),
            n_clusters,
            max_iter: 50,
            tolerance: 1e-4,
        }
    }

    pub fn with_max_iter(mut self, max_iter: usize) -> Self {
        self.max_iter = max_iter;
        self
    }

    // init centroids via k-means++
    fn init_centroids(&mut self, data: &[Vec<f32>]) {
        let mut rng = rand::thread_rng();

        if data.is_empty() {
            return;
        }

        self.centroids.clear();
        self.centroids.reserve(self.n_clusters);

        // premier centroid random
        let first_idx = (0..data.len()).collect::<Vec<_>>()
            .choose(&mut rng)
            .copied()
            .unwrap();
        self.centroids.push(data[first_idx].clone());

        // k-means++ pour les autres
        for _ in 1..self.n_clusters {
            let distances: Vec<f32> = data.par_iter()
                .map(|point| {
                    self.centroids.iter()
                        .map(|c| cosine_distance(point, c))
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap()
                })
                .collect();

            let total: f32 = distances.iter().sum();
            let mut r = rng.gen::<f32>() * total;

            let mut next_idx = 0;
            for (i, &d) in distances.iter().enumerate() {
                r -= d;
                if r <= 0.0 {
                    next_idx = i;
                    break;
                }
            }

            self.centroids.push(data[next_idx].clone());
        }
    }

    // assigner chaque point au centroid le plus proche
    fn assign_clusters(&self, data: &[Vec<f32>]) -> Vec<usize> {
        data.par_iter()
            .map(|point| {
                self.centroids.iter()
                    .enumerate()
                    .map(|(idx, c)| (idx, cosine_distance(point, c)))
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .map(|(idx, _)| idx)
                    .unwrap()
            })
            .collect()
    }

    // recalculer les centroids
    fn update_centroids(&mut self, data: &[Vec<f32>], assignments: &[usize]) -> f32 {
        let dim = data[0].len();
        let mut new_centroids = vec![vec![0.0; dim]; self.n_clusters];
        let mut counts = vec![0; self.n_clusters];

        for (point, &cluster) in data.iter().zip(assignments.iter()) {
            for (i, &val) in point.iter().enumerate() {
                new_centroids[cluster][i] += val;
            }
            counts[cluster] += 1;
        }

        // normaliser
        for (cluster_idx, count) in counts.iter().enumerate() {
            if *count > 0 {
                let c = *count as f32;
                for val in &mut new_centroids[cluster_idx] {
                    *val /= c;
                }
            }
        }

        // calculer le changement
        let mut total_shift = 0.0;
        for (old, new) in self.centroids.iter().zip(new_centroids.iter()) {
            total_shift += cosine_distance(old, new);
        }

        self.centroids = new_centroids;
        total_shift
    }

    pub fn fit(&mut self, data: &[Vec<f32>]) {
        if data.len() < self.n_clusters {
            // pas assez de data pour k clusters
            self.n_clusters = data.len();
        }

        self.init_centroids(data);

        for _ in 0..self.max_iter {
            let assignments = self.assign_clusters(data);
            let shift = self.update_centroids(data, &assignments);

            if shift < self.tolerance {
                break;
            }
        }
    }

    // trouver le cluster le plus proche
    pub fn predict(&self, point: &[f32]) -> usize {
        self.centroids.iter()
            .enumerate()
            .map(|(idx, c)| (idx, cosine_distance(point, c)))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kmeans_basic() {
        let data = vec![
            vec![1.0, 0.0],
            vec![0.9, 0.1],
            vec![0.0, 1.0],
            vec![0.1, 0.9],
        ];

        let mut kmeans = KMeans::new(2);
        kmeans.fit(&data);

        assert_eq!(kmeans.centroids.len(), 2);
    }
}
