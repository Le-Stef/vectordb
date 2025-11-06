use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use vectordb_rust::{VectorDbClient, VectorDbError};

type SharedClient = Arc<VectorDbClient>;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

struct AppError(VectorDbError);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            VectorDbError::CollectionNotFound(_) => (StatusCode::NOT_FOUND, self.0.to_string()),
            VectorDbError::CollectionAlreadyExists(_) => {
                (StatusCode::CONFLICT, self.0.to_string())
            }
            VectorDbError::VectorNotFound(_) => (StatusCode::NOT_FOUND, self.0.to_string()),
            VectorDbError::DimensionMismatch { .. } => {
                (StatusCode::BAD_REQUEST, self.0.to_string())
            }
            VectorDbError::InvalidConfig(_) => (StatusCode::BAD_REQUEST, self.0.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()),
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    } 
}

impl From<VectorDbError> for AppError {
    fn from(err: VectorDbError) -> Self {
        AppError(err)
    }
}

type AppResult<T> = Result<T, AppError>;

#[derive(Deserialize)]
struct CreateCollectionRequest {
    name: String,
    dimension: usize,
    #[serde(default)]
    use_ivf: bool,
    #[serde(default = "default_n_clusters")]
    n_clusters: usize,
}

fn default_n_clusters() -> usize {
    100
}

#[derive(Deserialize)]
struct AddRequest {
    ids: Vec<String>,
    embeddings: Vec<Vec<f32>>,
    metadatas: Option<Vec<HashMap<String, serde_json::Value>>>,
}

#[derive(Deserialize)]
struct GetRequest {
    ids: Option<Vec<String>>,
    include: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct UpdateRequest {
    ids: Vec<String>,
    metadatas: Vec<HashMap<String, serde_json::Value>>,
}

#[derive(Deserialize)]
struct DeleteRequest {
    ids: Vec<String>,
}

#[derive(Deserialize)]
struct QueryRequest {
    query_embedding: Vec<f32>,
    n_results: usize,
    #[serde(rename = "where")]
    where_filter: Option<vectordb_rust::filter::WhereFilter>,
}

fn convert_metadata(value: serde_json::Value) -> vectordb_rust::vector::MetadataValue {
    use vectordb_rust::vector::MetadataValue;
    match value {
        serde_json::Value::String(s) => MetadataValue::String(s),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                MetadataValue::Int(i)
            } else {
                MetadataValue::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::Bool(b) => MetadataValue::Bool(b),
        _ => MetadataValue::String(value.to_string()),
    }
}

async fn create_collection(
    State(client): State<SharedClient>,
    Json(req): Json<CreateCollectionRequest>,
) -> AppResult<Json<serde_json::Value>> {
    tracing::info!(
        name = %req.name,
        dimension = req.dimension,
        use_ivf = req.use_ivf,
        "Creating collection"
    );

    if req.use_ivf {
        client.create_collection_with_ivf(req.name.clone(), req.dimension, req.n_clusters)?;
    } else {
        client.create_collection(req.name.clone(), req.dimension)?;
    }

    Ok(Json(serde_json::json!({
        "status": "created",
        "name": req.name,
        "use_ivf": req.use_ivf,
        "n_clusters": if req.use_ivf { req.n_clusters } else { 0 }
    })))
}

async fn list_collections(
    State(client): State<SharedClient>,
) -> AppResult<Json<Vec<String>>> {
    let collections = client.list_collections()?;
    Ok(Json(collections))
}

async fn get_collection(
    State(client): State<SharedClient>,
    Path(name): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    client.get_collection(&name)?;
    Ok(Json(serde_json::json!({
        "status": "loaded",
        "name": name
    })))
}

async fn get_collection_stats(
    State(client): State<SharedClient>,
    Path(name): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let stats = client.with_collection(&name, |coll| coll.stats())?;
    Ok(Json(serde_json::to_value(&stats).unwrap()))
}

async fn begin_batch(
    State(client): State<SharedClient>,
    Path(name): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    client.with_collection_mut(&name, |coll| {
        coll.begin_batch();
        Ok(())
    })?;
    Ok(Json(serde_json::json!({"status": "batch_started"})))
}

async fn end_batch(
    State(client): State<SharedClient>,
    Path(name): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    client.with_collection_mut(&name, |coll| {
        coll.end_batch();
        Ok(())
    })?;
    Ok(Json(serde_json::json!({"status": "batch_ended"})))
}

async fn rebuild_index(
    State(client): State<SharedClient>,
    Path(name): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    use std::time::Instant;

    tracing::info!(collection = %name, "Rebuilding IVF index");

    let start = Instant::now();
    let stats = client.with_collection_mut(&name, |coll| {
        if !coll.config.use_ivf {
            return Err(VectorDbError::InvalidConfig(
                "Collection does not use IVF index".to_string()
            ));
        }
        coll.rebuild_index();
        Ok(coll.stats())
    })?;

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    tracing::info!(
        collection = %name,
        elapsed_ms = elapsed_ms,
        "IVF index rebuilt"
    );

    Ok(Json(serde_json::json!({
        "status": "rebuilt",
        "elapsed_ms": elapsed_ms,
        "collection_stats": stats
    })))
}

async fn health_check(State(client): State<SharedClient>) -> Json<serde_json::Value> {
    let collections = client.list_collections().unwrap_or_default();
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "collections_count": collections.len(),
    }))
}

async fn delete_collection(
    State(client): State<SharedClient>,
    Path(name): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    tracing::info!(collection = %name, "Deleting collection");
    client.delete_collection(&name)?;
    Ok(Json(serde_json::json!({
        "status": "deleted",
        "name": name
    })))
}

async fn add_vectors(
    State(client): State<SharedClient>,
    Path(collection_name): Path<String>,
    Json(req): Json<AddRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let count = req.ids.len();
    tracing::debug!(
        collection = %collection_name,
        count = count,
        "Adding vectors"
    );

    let metas = req.metadatas.map(|ms| {
        ms.into_iter()
            .map(|m| m.into_iter().map(|(k, v)| (k, convert_metadata(v))).collect())
            .collect()
    });

    client.with_collection_mut(&collection_name, |coll| {
        coll.add(req.ids.clone(), req.embeddings, metas)
    })?;

    Ok(Json(serde_json::json!({"status": "added", "count": count})))
}

async fn get_vectors(
    State(client): State<SharedClient>,
    Path(collection_name): Path<String>,
    Json(req): Json<GetRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let result = client.with_collection(&collection_name, |coll| -> Result<_, VectorDbError> {
        coll.get(req.ids, req.include)
    })??;

    Ok(Json(serde_json::to_value(&result).unwrap()))
}

async fn update_vectors(
    State(client): State<SharedClient>,
    Path(collection_name): Path<String>,
    Json(req): Json<UpdateRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let metas: Vec<HashMap<String, _>> = req
        .metadatas
        .into_iter()
        .map(|meta| meta.into_iter().map(|(k, v)| (k, convert_metadata(v))).collect())
        .collect();

    let n = req.ids.len();
    client.with_collection_mut(&collection_name, |coll| coll.update(req.ids.clone(), metas))?;

    Ok(Json(serde_json::json!({"status": "updated", "count": n})))
}

async fn delete_vectors(
    State(client): State<SharedClient>,
    Path(collection_name): Path<String>,
    Json(req): Json<DeleteRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let count = req.ids.len();
    client.with_collection_mut(&collection_name, |coll| coll.delete(req.ids))?;
    Ok(Json(serde_json::json!({"status": "deleted", "count": count})))
}

async fn query_vectors(
    State(client): State<SharedClient>,
    Path(coll_name): Path<String>,
    Json(req): Json<QueryRequest>,
) -> AppResult<Json<serde_json::Value>> {
    tracing::debug!(
        collection = %coll_name,
        n_results = req.n_results,
        has_filter = req.where_filter.is_some(),
        "Querying vectors"
    );

    let results = client.with_collection_mut(&coll_name, |coll| {
        coll.query(&req.query_embedding, req.n_results, req.where_filter.as_ref())
    })?;

    tracing::debug!(
        collection = %coll_name,
        results_count = results.len(),
        "Query completed"
    );

    Ok(Json(serde_json::to_value(&results).unwrap()))
}

#[tokio::main]
async fn main() {
    // initialiser tracing
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vectordb_rust=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_path = std::env::var("VECTORDB_PATH").unwrap_or("./vector_db".into());
    let mut port: u16 = std::env::var("VECTORDB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let client = Arc::new(VectorDbClient::new(&db_path).expect("Failed to create client"));
    tracing::info!("VectorDB client initialized at {}", db_path);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/collections", post(create_collection).get(list_collections))
        .route("/collections/:name", get(get_collection).delete(delete_collection))
        .route("/collections/:name/stats", get(get_collection_stats))
        .route("/collections/:name/batch/begin", post(begin_batch))
        .route("/collections/:name/batch/end", post(end_batch))
        .route("/collections/:name/rebuild", post(rebuild_index))
        .route("/collections/:name/add", post(add_vectors))
        .route("/collections/:name/get", post(get_vectors))
        .route("/collections/:name/update", put(update_vectors))
        .route("/collections/:name/delete", delete(delete_vectors))
        .route("/collections/:name/query", post(query_vectors))
        .layer(CorsLayer::permissive())
        .with_state(client);

    // essayer plusieurs ports si occupé
    let listener = loop {
        let addr = format!("0.0.0.0:{}", port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => {
                tracing::info!("VectorDB server started on {}", addr);
                break l;
            }
            Err(e) => {
                tracing::warn!("Port {} unavailable: {}", port, e);
                if port >= 8090 {
                    tracing::error!("Could not bind to any port between 8080-8090");
                    panic!("Could not bind to any port between 8080-8090");
                }
                port += 1;
            }
        }
    };

    axum::serve(listener, app).await.unwrap();
}
