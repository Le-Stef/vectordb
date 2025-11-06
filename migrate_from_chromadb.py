#!/usr/bin/env python3
"""
Script de migration depuis ChromaDB vers VectorDB Rust
Usage: python migrate_from_chromadb.py [--chromadb-path PATH] [--vectordb-host HOST] [--vectordb-port PORT]
"""

import argparse
import sys
from typing import List
import chromadb
from vectordb_client import PersistentClient
from tqdm import tqdm


def migrate_collection(
    chroma_collection,
    vectordb_client: PersistentClient,
    batch_size: int = 1000,
    use_ivf: bool = False,
):
    """Migre une collection ChromaDB vers VectorDB"""

    name = chroma_collection.name
    print(f"\n📦 Migration de la collection: {name}")

    # récupérer tous les vecteurs
    result = chroma_collection.get(include=["embeddings", "metadatas"])

    if not result["ids"]:
        print(f"   ⚠️  Collection vide, ignorée")
        return

    ids = result["ids"]
    embeddings = result["embeddings"]
    metadatas = result.get("metadatas", [None] * len(ids))

    total = len(ids)
    dimension = len(embeddings[0]) if embeddings else 0

    print(f"   Dimension: {dimension}, Vecteurs: {total}")

    # créer la collection dans VectorDB
    try:
        n_clusters = max(10, int(total ** 0.5)) if use_ivf else 100
        coll = vectordb_client.create_collection(
            name=name,
            dimension=dimension,
            use_ivf=use_ivf,
            n_clusters=n_clusters
        )
    except Exception as e:
        if "409" in str(e):  # collection existe déjà
            print(f"   Collection existe déjà, tentative de récupération...")
            coll = vectordb_client.get_collection(name)
        else:
            raise

    # migration par batch
    print(f"   Migration en cours...")

    with coll.batch():
        for i in tqdm(range(0, total, batch_size), desc="   Batches"):
            end = min(i + batch_size, total)

            batch_ids = ids[i:end]
            batch_embeddings = embeddings[i:end]
            batch_metadatas = metadatas[i:end] if metadatas else None

            # nettoyer les métadonnées None
            if batch_metadatas:
                batch_metadatas = [m if m is not None else {} for m in batch_metadatas]

            coll.add(
                ids=batch_ids,
                embeddings=batch_embeddings,
                metadatas=batch_metadatas
            )

    print(f"   ✅ {total} vecteurs migrés")


def main():
    parser = argparse.ArgumentParser(
        description="Migre des collections ChromaDB vers VectorDB Rust"
    )
    parser.add_argument(
        "--chromadb-path",
        default="./chroma_db",
        help="Chemin vers la base ChromaDB (défaut: ./chroma_db)"
    )
    parser.add_argument(
        "--vectordb-host",
        default="localhost",
        help="Hôte du serveur VectorDB (défaut: localhost)"
    )
    parser.add_argument(
        "--vectordb-port",
        type=int,
        default=8080,
        help="Port du serveur VectorDB (défaut: 8080)"
    )
    parser.add_argument(
        "--collections",
        nargs="+",
        help="Noms des collections à migrer (par défaut: toutes)"
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=1000,
        help="Taille des batches pour la migration (défaut: 1000)"
    )
    parser.add_argument(
        "--use-ivf",
        action="store_true",
        help="Utiliser l'index IVF pour les collections (recommandé pour >10k vecteurs)"
    )

    args = parser.parse_args()

    print("🚀 Démarrage de la migration ChromaDB → VectorDB")
    print(f"   Source: {args.chromadb_path}")
    print(f"   Destination: {args.vectordb_host}:{args.vectordb_port}")
    print(f"   IVF: {'Activé' if args.use_ivf else 'Désactivé'}")

    # connexion à ChromaDB
    try:
        chroma_client = chromadb.PersistentClient(path=args.chromadb_path)
        chroma_collections = chroma_client.list_collections()
        print(f"\n✅ ChromaDB connecté: {len(chroma_collections)} collection(s) trouvée(s)")
    except Exception as e:
        print(f"\n❌ Erreur de connexion à ChromaDB: {e}")
        sys.exit(1)

    # connexion à VectorDB
    try:
        vectordb_client = PersistentClient(
            host=args.vectordb_host,
            port=args.vectordb_port
        )
        print(f"✅ VectorDB connecté")
    except Exception as e:
        print(f"\n❌ Erreur de connexion à VectorDB: {e}")
        print(f"   Assurez-vous que le serveur est démarré avec: cargo run --bin vectordb_server")
        sys.exit(1)

    # filtrer les collections si spécifié
    if args.collections:
        collections_to_migrate = [
            c for c in chroma_collections if c.name in args.collections
        ]
        if not collections_to_migrate:
            print(f"\n❌ Aucune collection trouvée parmi: {args.collections}")
            sys.exit(1)
    else:
        collections_to_migrate = chroma_collections

    # migration
    print(f"\n📊 {len(collections_to_migrate)} collection(s) à migrer")

    success_count = 0
    error_count = 0

    for chroma_coll in collections_to_migrate:
        try:
            migrate_collection(
                chroma_coll,
                vectordb_client,
                batch_size=args.batch_size,
                use_ivf=args.use_ivf
            )
            success_count += 1
        except Exception as e:
            print(f"   ❌ Erreur: {e}")
            error_count += 1

    # résumé
    print(f"\n{'='*60}")
    print(f"✅ Migration terminée:")
    print(f"   Succès: {success_count}")
    print(f"   Erreurs: {error_count}")
    print(f"{'='*60}")

    if error_count > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
