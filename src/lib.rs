pub mod collection;
pub mod vector;
pub mod distance;
pub mod storage;
pub mod error;
pub mod client;
pub mod kmeans;
pub mod ivf;
pub mod filter;

pub use collection::Collection;
pub use client::VectorDbClient;
pub use error::{VectorDbError, Result};

// exposer pour les benchmarks
pub use distance::dot_product;
