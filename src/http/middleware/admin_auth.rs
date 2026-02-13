use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

pub async fn require_internal_api_key(
    State(expected): State<String>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let provided = request
        .headers()
        .get("X-Internal-Api-Key")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if provided != expected {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::from("unauthorized"))
            .unwrap_or_else(|_| Response::new(Body::from("unauthorized")));
    }

    next.run(request).await
}
