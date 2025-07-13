// src/api/handlers.rs
use warp::{http::StatusCode, Rejection, Reply};
use serde_json::json;

// CORS 处理
pub fn cors() -> warp::cors::Builder {
    warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type", "authorization"])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
}

// 错误处理
pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not Found";
    } else if err.find::<warp::filters::body::BodyDeserializeError>().is_some() {
        code = StatusCode::BAD_REQUEST;
        message = "Invalid JSON body";
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "Method Not Allowed";
    } else if let Some(auth_error) = err.find::<crate::api::auth::AuthError>() {
        code = StatusCode::UNAUTHORIZED;
        message = match auth_error {
            crate::api::auth::AuthError::InvalidToken => "Invalid JWT token",
            crate::api::auth::AuthError::MissingToken => "Missing Authorization header",
            crate::api::auth::AuthError::SessionExpired => "Session expired",
        };
    } else {
        tracing::error!("Unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal Server Error";
    }

    let json = json!({
        "success": false,
        "message": message,
    });

    Ok(warp::reply::with_status(
        warp::reply::json(&json),
        code,
    ))
}

// 日志中间件
pub fn with_logging() -> warp::log::Log<impl Fn(warp::log::Info) + Copy> {
    warp::log::custom(|info| {
        tracing::info!(
            method = %info.method(),
            path = %info.path(),
            status = %info.status(),
            elapsed = ?info.elapsed(),
            remote_addr = ?info.remote_addr(),
            "API request"
        );
    })
}