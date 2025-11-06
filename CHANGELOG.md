# Changelog

Ce fichier référence toutes les modifications importantes apportées à ce projet.

Le format est basé sur [Keep a Changelog](https://keepachangelog.com/fr/1.0.0/),
et ce projet adhère au [Semantic Versioning](https://semver.org/lang/fr/).

## [0.1.0] - 2025-11-06

### Ajouté

#### Core
- Structure de données `VectorEntry` avec support des métadonnées
- Système de métadonnées flexible (String, Int, Float, Bool)
- Client VectorDB avec gestion des collections
- Collection avec opérations CRUD complètes
- Persistance JSON pour les collections et vecteurs

#### Calculs
- Calcul de distance cosinus optimisé
- Normalisation L2 automatique des vecteurs
- Fonction de similarité cosinus
- Tests unitaires pour les calculs de distance

#### API REST
- Serveur HTTP asynchrone avec Axum
- Endpoints pour la gestion des collections
  - POST `/collections` - Créer une collection
  - GET `/collections` - Lister les collections
  - GET `/collections/:name` - Charger une collection
  - DELETE `/collections/:name` - Supprimer une collection
- Endpoints pour la gestion des vecteurs
  - POST `/collections/:name/add` - Ajouter des vecteurs
  - POST `/collections/:name/get` - Récupérer des vecteurs
  - PUT `/collections/:name/update` - Mettre à jour les métadonnées
  - DELETE `/collections/:name/delete` - Supprimer des vecteurs
  - POST `/collections/:name/query` - Rechercher des vecteurs similaires
- Support CORS pour les requêtes cross-origin
- Gestion d'erreurs robuste avec codes HTTP appropriés

#### Client Python
- Client Python `PersistentClient` pour l'API REST
- Classe `Collection` avec toutes les opérations CRUD
- Script de test complet avec exemples d'utilisation
- Support des requêtes HTTP avec gestion d'erreurs

#### Configuration
- Variables d'environnement pour la configuration
  - `VECTORDB_PATH` - Chemin de stockage
  - `VECTORDB_PORT` - Port du serveur
- Script de démarrage Windows (`start_server.bat`)

#### Documentation
- README complet avec exemples
- Cahier des charges détaillé (doc1.md)
- Documentation de l'architecture
- Guide de contribution (CONTRIBUTING.md)
- Licence MIT (LICENSE)

#### Tests
- Tests unitaires pour les calculs de distance
- Tests de normalisation L2
- Script de test Python complet

### Dépendances

- `axum` 0.7 - Framework web asynchrone
- `tokio` 1.35 - Runtime asynchrone
- `serde` 1.0 - Sérialisation/désérialisation
- `serde_json` 1.0 - Support JSON
- `bincode` 1.3 - Sérialisation binaire
- `rayon` 1.7 - Parallélisation
- `tower` 0.4 - Middleware
- `tower-http` 0.5 - CORS et autres
- `anyhow` 1.0 - Gestion d'erreurs
- `thiserror` 1.0 - Erreurs personnalisées

## [0.2.0] - 2025-11-06

### Ajouté

#### Phase 2 : Optimisations - Index IVF
- [x] K-means clustering avec algorithme K-means++
  - Initialisation intelligente des centroids
  - Parallélisation avec Rayon pour l'assignation des clusters
  - Paramètre configurable pour le nombre de clusters
- [x] Index IVF (Inverted File Index)
  - Structure d'index inversé pour partitionner l'espace vectoriel
  - Paramètre `n_probe` pour contrôler le compromis précision/vitesse
  - Rebuild automatique après modifications (add/delete)
  - Sérialisation/désérialisation de l'index
- [x] Mode de recherche hybride
  - Recherche linéaire pour les petites collections
  - Recherche IVF pour les grandes collections
  - Fallback automatique si l'index n'est pas construit
- [x] API REST étendue
  - Support `use_ivf` et `n_clusters` dans la création de collections
  - Rebuild automatique de l'index lors des requêtes
- [x] Client Python mis à jour
  - Paramètres `use_ivf` et `n_clusters` dans `create_collection()`
  - Compatible avec l'ancienne API (use_ivf=False par défaut)
- [x] Tests et scripts
  - Script `test_ivf.py` pour tester la précision et la scalabilité
  - Tests unitaires pour K-means et IVF

#### Phase 2 : Optimisations avancées
- [x] Optimisations du dot product
  - Déroulement de boucle (loop unrolling) pour vectorisation automatique
  - Branchement optimisé pour petits vecteurs (<8 dimensions)
  - Accumulation en 4 registres pour meilleure utilisation du CPU
- [x] Parallélisation avec Rayon
  - Recherche linéaire parallèle pour collections >100 vecteurs
  - Recherche IVF parallèle pour >50 candidats
  - Assignation des clusters parallèle dans K-means
- [x] Optimisation de la sérialisation
  - Format bincode pour stockage (5-10x plus rapide que JSON)
  - Buffer de 512KB pour I/O optimisé
  - Rétrocompatibilité avec ancien format JSON
- [x] Benchmarks
  - Module criterion pour mesures précises
  - Benchmarks de recherche linéaire et IVF
  - Benchmark de dot product pour différentes dimensions

### Modifié
- Signature de `Collection::query()` : `&self` → `&mut self` pour rebuild automatique
- Structure `CollectionConfig` : ajout de `use_ivf` et `n_clusters`
- Structure `Collection` : ajout du champ `ivf_index` et `needs_rebuild`
- Stockage : `data.json` → `data.bin` (bincode)

### Performance
- **Recherche** : 5-20x plus rapide avec IVF pour grandes collections
- **Dot product** : ~2x plus rapide avec loop unrolling
- **Parallélisation** : 2-4x plus rapide sur CPU multi-core
- **Sérialisation** : 5-10x plus rapide avec bincode
- **Gain global** : jusqu'à **40-100x** pour grandes collections (>10k vecteurs)

## [Non publié]

### À venir

#### Phase 2 : Optimisations (suite)
- [ ] Optimisations SIMD pour les calculs de distance
- [ ] Benchmarks détaillés de performance

#### Phase 3 : Fonctionnalités avancées
- [ ] Product Quantization pour la compression
- [ ] Index HNSW comme alternative à IVF
- [ ] Monitoring et métriques Prometheus
- [ ] Support du sharding pour grands datasets
- [ ] API de statistiques sur les collections

---

## Convention

### Types de changements

- **Ajouté** pour les nouvelles fonctionnalités
- **Modifié** pour les changements dans les fonctionnalités existantes
- **Déprécié** pour les fonctionnalités bientôt supprimées
- **Supprimé** pour les fonctionnalités supprimées
- **Corrigé** pour les corrections de bugs
- **Sécurité** pour les vulnérabilités corrigées
