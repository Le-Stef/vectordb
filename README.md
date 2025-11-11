# VectorDB Rust

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://rust-lang.org/fr/)

> Base de données vectorielle haute performance en Rust avec index IVF et API REST

Une implémentation légère et performante d'une base de données vectorielle, offrant une alternative à ChromaDB.

## Caractéristiques

- **Performant** : Rust + Rayon pour calcul parallèle
- **Index IVF** : Recherche approximative en O(√n) pour grandes collections (>10k vecteurs)
- **Batch operations** : Import massif optimisé
- **Filtrage avancé** : Opérateurs `$ne`, `$in`, `$nin` sur métadonnées
- **Cache LRU** : Gestion mémoire intelligente avec lazy loading
- **Logging structuré** : Monitoring avec `tracing`
- **Compatible ChromaDB** : Migration facile avec API similaire
- **Client Python** : Interface simple et intuitive
- **API REST** : Serveur HTTP asynchrone avec Axum

## Démarrage rapide

### Installation

```bash
# Cloner le dépôt
git clone https://github.com/VOTRE-USERNAME/vectordb-rust.git
cd vectordb-rust

# Compiler
cargo build --release

# Démarrer le serveur
cargo run --release --bin vectordb_server
```

Le serveur démarre sur `http://localhost:8080` ou un autre port si celui-ci est indisponible

### Utilisation avec Python

```bash
# Installer le client Python
pip install -r requirements.txt
```

```python
from vectordb_client import PersistentClient

# Connexion
client = PersistentClient()

# Créer une collection avec IVF (recommandé pour >10k vecteurs)
collection = client.create_collection(
    name="images",
    dimension=1280,
    use_ivf=True,
    n_clusters=100
)

# Ajouter des vecteurs
collection.add(
    ids=["img1", "img2", "img3"],
    embeddings=[[0.1, 0.2, ...], [0.3, 0.4, ...], [0.5, 0.6, ...]],
    metadatas=[
        {"source": "camera", "date": "2024-01-01"},
        {"source": "upload", "date": "2024-01-02"},
        {"source": "camera", "date": "2024-01-03"}
    ]
)

# Rechercher avec filtrage par métadonnées
results = collection.query(
    query_embedding=[0.1, 0.2, ...],
    n_results=10,
    where={"source": "camera"}
)

print(results)
```

## Performance

Benchmarks sur collection de 10,000 vecteurs (dimension 128) :

| Opération | Recherche linéaire | IVF (n_probe=4) | Speedup |
|-----------|-------------------|-----------------|---------|
| Query (k=10) | ~8.5ms | ~0.25ms | **34x** |
| Add 1000 vecteurs | 12ms | 15ms | 0.8x |

**Note** : IVF est plus lent sur petites collections (<1k vecteurs). Utilisez-le pour >10k vecteurs.

## Migration depuis ChromaDB

Script de migration automatique inclus :

```bash
# Installer les dépendances
pip install -r requirements.txt

# Migrer toutes les collections
python migrate_from_chromadb.py --chromadb-path ./chroma_db --use-ivf

# Migrer des collections spécifiques
python migrate_from_chromadb.py --collections images embeddings --use-ivf
```

## API REST

### Collections

```bash
# Créer une collection avec IVF
POST /collections
{
  "name": "images",
  "dimension": 1280,
  "use_ivf": true,
  "n_clusters": 100
}

# Lister les collections
GET /collections

# Statistiques
GET /collections/{name}/stats

# Supprimer
DELETE /collections/{name}
```

### Vecteurs

```bash
# Ajouter
POST /collections/{name}/add
{
  "ids": ["id1", "id2"],
  "embeddings": [[...], [...]],
  "metadatas": [{"key": "value"}, ...]
}

# Rechercher avec filtrage
POST /collections/{name}/query
{
  "query_embedding": [...],
  "n_results": 10,
  "where": {"source": "camera"}
}

# Obtenir
POST /collections/{name}/get
{
  "ids": ["id1", "id2"],
  "include": ["embeddings", "metadatas"]
}

# Mettre à jour métadonnées
PUT /collections/{name}/update
{
  "ids": ["id1"],
  "metadatas": [{"new_key": "new_value"}]
}

# Supprimer
DELETE /collections/{name}/delete
{
  "ids": ["id1", "id2"]
}
```

### Batch & Rebuild

```bash
# Mode batch (désactive rebuild automatique)
POST /collections/{name}/batch/begin
# ... ajouter beaucoup de vecteurs ...
POST /collections/{name}/batch/end

# Rebuild manuel de l'index IVF
POST /collections/{name}/rebuild

# Health check
GET /health
```

## Configuration

Variables d'environnement :

```bash
VECTORDB_PATH=/chemin/vers/db     # Chemin de stockage (défaut: ./vector_db)
VECTORDB_PORT=8080                # Port du serveur (défaut: 8080)
VECTORDB_MAX_CACHED=20            # Nombre max de collections en cache (défaut: 20)
RUST_LOG=info                     # Niveau de logs (debug, info, warn, error)
```

## Tests et Benchmarks

```bash
# Tests unitaires
cargo test

# Benchmarks
cargo bench

# Linter
cargo clippy
```

## Structure du projet

```
vectordb-rust/
├── src/
│   ├── main.rs           # Serveur API REST
│   ├── lib.rs            # Exports publics
│   ├── collection.rs     # Gestion des collections
│   ├── client.rs         # Client avec cache LRU
│   ├── storage.rs        # Persistance bincode
│   ├── ivf.rs            # Index IVF
│   ├── kmeans.rs         # Clustering K-means++
│   ├── distance.rs       # Calculs optimisés
│   ├── filter.rs         # Filtrage métadonnées
│   └── error.rs          # Gestion d'erreurs
├── benches/              # Benchmarks
├── vectordb_client.py    # Client Python
├── migrate_from_chromadb.py  # Script de migration
└── requirements.txt      # Dépendances Python
```


## Licence

Licence Apache 2.0 - voir le fichier [LICENSE](LICENSE)

---

**Développé avec ❤️ en Rust**
