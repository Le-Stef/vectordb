"""
Client Python pour VectorDB Rust
Compatible avec l'API ChromaDB pour faciliter la migration
"""

import requests
import json
from typing import List, Dict, Optional, Any


class Collection:
    def __init__(self, name: str, base_url: str):
        self.name = name
        self.base_url = base_url

    def add(
        self,
        ids: List[str],
        embeddings: List[List[float]],
        metadatas: Optional[List[Dict[str, Any]]] = None,
        documents: Optional[List[str]] = None,  # Ignoré, pour compatibilité ChromaDB
    ):
        url = f"{self.base_url}/collections/{self.name}/add"
        data = {
            "ids": ids,
            "embeddings": embeddings,
        }
        if metadatas is not None:
            data["metadatas"] = metadatas

        response = requests.post(url, json=data)
        response.raise_for_status()
        return response.json()

    def get(
        self,
        ids: Optional[List[str]] = None,
        include: Optional[List[str]] = None,
    ):
        url = f"{self.base_url}/collections/{self.name}/get"
        data = {}
        if ids is not None:
            data["ids"] = ids
        if include is not None:
            data["include"] = include

        response = requests.post(url, json=data)
        response.raise_for_status()
        return response.json()

    def update(
        self,
        ids: List[str],
        metadatas: List[Dict[str, Any]],
    ):
        url = f"{self.base_url}/collections/{self.name}/update"
        data = {
            "ids": ids,
            "metadatas": metadatas,
        }

        response = requests.put(url, json=data)
        response.raise_for_status()
        return response.json()

    def delete(self, ids: List[str]):
        url = f"{self.base_url}/collections/{self.name}/delete"
        data = {"ids": ids}

        response = requests.delete(url, json=data)
        response.raise_for_status()
        return response.json()

    def query(
        self,
        query_embedding: List[float],
        n_results: int = 10,
        where: Optional[Dict[str, Any]] = None,
    ):
        url = f"{self.base_url}/collections/{self.name}/query"
        data = {
            "query_embedding": query_embedding,
            "n_results": n_results,
        }

        if where is not None:
            data["where"] = where

        response = requests.post(url, json=data)
        response.raise_for_status()
        return response.json()

    def count(self):
        """Compte le nombre d'éléments dans la collection"""
        result = self.get(include=[])
        return len(result.get('ids', []))

    def stats(self):
        """Retourne les statistiques de la collection"""
        url = f"{self.base_url}/collections/{self.name}/stats"
        response = requests.get(url)
        response.raise_for_status()
        return response.json()

    def begin_batch(self):
        """Démarre un mode batch (pas de rebuild IVF automatique)"""
        url = f"{self.base_url}/collections/{self.name}/batch/begin"
        response = requests.post(url)
        response.raise_for_status()
        return response.json()

    def end_batch(self):
        """Termine le mode batch et rebuild l'index si nécessaire"""
        url = f"{self.base_url}/collections/{self.name}/batch/end"
        response = requests.post(url)
        response.raise_for_status()
        return response.json()

    def rebuild_index(self):
        """Force un rebuild de l'index IVF"""
        url = f"{self.base_url}/collections/{self.name}/rebuild"
        response = requests.post(url)
        response.raise_for_status()
        return response.json()

    def batch(self):
        """Context manager pour mode batch"""
        return BatchContext(self)


class BatchContext:
    def __init__(self, collection):
        self.collection = collection

    def __enter__(self):
        self.collection.begin_batch()
        return self.collection

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.collection.end_batch()
        return False


class PersistentClient:
    def __init__(self, path: str = "./vector_db", host: str = "localhost", port: int = 8080):
        self.path = path
        self.base_url = f"http://{host}:{port}"
        # Vérifier que le serveur est accessible
        try:
            requests.get(f"{self.base_url}/collections", timeout=2)
        except requests.exceptions.RequestException as e:
            raise ConnectionError(
                f"Impossible de se connecter au serveur VectorDB sur {self.base_url}. "
                f"Assurez-vous que le serveur est démarré avec: cargo run --bin vectordb_server"
            ) from e

    def create_collection(
        self,
        name: str,
        dimension: int = 1280,
        use_ivf: bool = False,
        n_clusters: int = 100
    ):
        url = f"{self.base_url}/collections"
        data = {
            "name": name,
            "dimension": dimension,
        }

        if use_ivf:
            data["use_ivf"] = True
            data["n_clusters"] = n_clusters

        try:
            response = requests.post(url, json=data)
            response.raise_for_status()
        except requests.exceptions.HTTPError as e:
            if e.response.status_code == 409:  # Collection existe déjà
                return self.get_collection(name)
            raise

        return Collection(name, self.base_url)

    def get_collection(self, name: str):
        url = f"{self.base_url}/collections/{name}"
        response = requests.get(url)
        response.raise_for_status()
        return Collection(name, self.base_url)

    def delete_collection(self, name: str):
        url = f"{self.base_url}/collections/{name}"
        response = requests.delete(url)
        response.raise_for_status()
        return response.json()

    def list_collections(self):
        url = f"{self.base_url}/collections"
        response = requests.get(url)
        response.raise_for_status()
        return response.json()
