//! 扩展 API:GET / 返回 API 元数据(meta、skinDomains、signaturePublickey)
//! 并设置 ALI 头 X-Authlib-Injector-API-Location

use axum::extract::State;
use axum::http::HeaderName;
use axum::response::IntoResponse;
use serde::Serialize;

use crate::app::state::AppState;
use crate::core::crypto::public_key_pem;
use crate::core::error::ApiResult;
use crate::core::types::JsonResponse;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    server_name: String,
    implementation_name: String,
    implementation_version: String,
    #[serde(rename = "feature.non_email_login")]
    feature_non_email_login: bool,
    #[serde(rename = "feature.enable_profile_key")]
    feature_enable_profile_key: bool,
    links: MetaLinks,
}

#[derive(Debug, Serialize)]
struct MetaLinks {
    homepage: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MetaResponse {
    meta: Meta,
    skin_domains: Vec<String>,
    signature_publickey: String,
}

/// GET / — authlib-injector API 元数据
pub async fn meta(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    let pem = public_key_pem(&state.public_key)
        .map_err(|e| crate::core::error::ApiError::internal(e.to_string()))?;

    let response = MetaResponse {
        meta: Meta {
            server_name: state.config.server_name.clone(),
            implementation_name: state.config.implementation_name.clone(),
            implementation_version: state.config.implementation_version.clone(),
            feature_non_email_login: state.config.non_email_login,
            feature_enable_profile_key: true,
            links: MetaLinks {
                homepage: state.config.base_url.clone(),
            },
        },
        skin_domains: state.config.texture_domains(),
        signature_publickey: pem,
    };

    Ok((
        [(
            HeaderName::from_static("x-authlib-injector-api-location"),
            "/",
        )],
        JsonResponse(response),
    ))
}
