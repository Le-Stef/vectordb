use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use vectordb_rust::Collection;
use rand::Rng;

fn generate_vectors(n: usize, dim: usize) -> Vec<Vec<f32>> {
    let mut rng = rand::thread_rng();
    (0..n)
        .map(|_| {
            let vec: Vec<f32> = (0..dim).map(|_| rng.gen::<f32>()).collect();
            // normaliser
            let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            vec.iter().map(|x| x / norm).collect()
        })
        .collect()
}

fn bench_linear_search(c: &mut Criterion) {
    let dim = 128;
    let sizes = vec![100, 500, 1000, 5000];

    let mut group = c.benchmark_group("linear_search");

    for size in sizes {
        let vectors = generate_vectors(size, dim);
        let ids: Vec<String> = (0..size).map(|i| format!("vec_{}", i)).collect();

        let mut coll = Collection::new("test".to_string(), dim);
        coll.add(ids.clone(), vectors.clone(), None).unwrap();

        let query = vectors[0].clone();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, _| {
                b.iter(|| {
                    coll.query(black_box(&query), black_box(10), None).unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_ivf_search(c: &mut Criterion) {
    let dim = 128;
    let sizes = vec![1000, 5000, 10000];

    let mut group = c.benchmark_group("ivf_search");

    for size in sizes {
        let vectors = generate_vectors(size, dim);
        let ids: Vec<String> = (0..size).map(|i| format!("vec_{}", i)).collect();

        let n_clusters = (size as f32).sqrt() as usize;
        let mut coll = Collection::new_with_ivf("test".to_string(), dim, n_clusters);
        coll.add(ids.clone(), vectors.clone(), None).unwrap();

        // Rebuild AVANT le benchmark
        coll.rebuild_index();

        // Forcer une première query pour s'assurer que tout est initialisé
        let query = vectors[0].clone();
        let _ = coll.query(&query, 10, None).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, _| {
                b.iter(|| {
                    // Maintenant le bench ne mesure que la query
                    coll.query(black_box(&query), black_box(10), None).unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_dot_product(c: &mut Criterion) {
    use vectordb_rust::distance::dot_product;

    let dims = vec![128, 512, 1280, 2048];
    let mut group = c.benchmark_group("dot_product");

    for dim in dims {
        let a = generate_vectors(1, dim)[0].clone();
        let b = generate_vectors(1, dim)[0].clone();

        group.bench_with_input(
            BenchmarkId::from_parameter(dim),
            &dim,
            |bench, _| {
                bench.iter(|| {
                    dot_product(black_box(&a), black_box(&b))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_linear_search, bench_ivf_search, bench_dot_product);
criterion_main!(benches);
