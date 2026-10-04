//! The cockpit's model picker reads the shared models.dev snapshot.

use super::*;

#[derive(Deserialize)]
pub(super) struct CatalogQuery {
    #[serde(default)]
    refresh: bool,
}

pub(super) async fn get_catalog(Query(query): Query<CatalogQuery>) -> Response<Body> {
    match crate::model_catalog::refresh(query.refresh).await {
        Some(body) => (
            [
                (CONTENT_TYPE, HeaderValue::from_static("application/json")),
                (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            ],
            Body::from(body),
        )
            .into_response(),
        None => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "model_catalog",
            "Model suggestions are temporarily unavailable.",
        ),
    }
}

