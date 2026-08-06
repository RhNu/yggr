//! Yggdrasil 错误响应格式:
//! ```json
//! { "error": "机器可读的简短描述", "errorMessage": "人类可读的详细信息", "cause": "原因(可选)" }
//! ```

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::{error, warn};

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub error: String,
    pub error_message: String,
    pub cause: Option<String>,
}

impl ApiError {
    pub fn new(status: StatusCode, error: &str, message: impl Into<String>) -> Self {
        ApiError {
            status,
            error: error.to_string(),
            error_message: message.into(),
            cause: None,
        }
    }

    /// 令牌无效(403 ForbiddenOperationException / Invalid token.)
    pub fn invalid_token() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "ForbiddenOperationException",
            "Invalid token.",
        )
    }

    /// 密码错误(403 ForbiddenOperationException)
    pub fn invalid_credentials() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "ForbiddenOperationException",
            "Invalid credentials. Invalid username or password.",
        )
    }

    /// 试图向已绑定角色的令牌指定角色(400 IllegalArgumentException)
    pub fn profile_already_assigned() -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "IllegalArgumentException",
            "Access token already has a profile assigned.",
        )
    }

    /// 试图选择一个不属于自己的角色(403 ForbiddenOperationException)
    pub fn invalid_profile_selection() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "ForbiddenOperationException",
            "Profile is not owned by the user.",
        )
    }

    /// 通用业务错误(400 IllegalArgumentException)
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "IllegalArgumentException", message)
    }

    /// 未认证(401,材质上传等)
    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "Unauthorized", "Unauthorized")
    }

    /// 角色不存在(404,材质上传等)
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NotFound", message)
    }

    /// 内部错误(500)
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "InternalServerError",
            message,
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": self.error,
            "errorMessage": self.error_message,
            "cause": self.cause,
        });
        if self.status.is_server_error() {
            error!(
                status = %self.status,
                error = %self.error,
                message = %self.error_message,
                "server error response"
            );
        } else if self.status.is_client_error() {
            warn!(
                status = %self.status,
                error = %self.error,
                message = %self.error_message,
                "client error response"
            );
        }
        (
            self.status,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            )],
            serde_json::to_vec(&body).unwrap_or_default(),
        )
            .into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
