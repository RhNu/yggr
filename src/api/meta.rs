//! 扩展 API:GET /service 返回 API 元数据(meta、skinDomains、signaturePublickey)
//! ALI 头在根路径 GET / 与此端点均可指向自身

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
    #[serde(
        rename = "feature.legacy_skin_api",
        skip_serializing_if = "Option::is_none"
    )]
    feature_legacy_skin_api: Option<bool>,
    #[serde(
        rename = "feature.no_mojang_namespace",
        skip_serializing_if = "Option::is_none"
    )]
    feature_no_mojang_namespace: Option<bool>,
    #[serde(
        rename = "feature.enable_mojang_anti_features",
        skip_serializing_if = "Option::is_none"
    )]
    feature_enable_mojang_anti_features: Option<bool>,
    #[serde(
        rename = "feature.username_check",
        skip_serializing_if = "Option::is_none"
    )]
    feature_username_check: Option<bool>,
    links: MetaLinks,
}

#[derive(Debug, Serialize)]
struct MetaLinks {
    homepage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    register: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MetaResponse {
    meta: Meta,
    skin_domains: Vec<String>,
    signature_publickey: String,
}

/// GET /service - authlib-injector API 元数据
pub async fn meta(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    let pem = public_key_pem(&state.public_key)
        .map_err(|e| crate::core::error::ApiError::internal(e.to_string()))?;

    let meta = Meta {
        server_name: state.config.server.name.clone(),
        implementation_name: state.config.meta.implementation_name.clone(),
        implementation_version: state.config.meta.implementation_version.clone(),
        feature_non_email_login: state.config.auth.non_email_login,
        feature_enable_profile_key: true,
        feature_legacy_skin_api: if state.config.features.legacy_skin_api {
            Some(true)
        } else {
            None
        },
        feature_no_mojang_namespace: if state.config.features.no_mojang_namespace {
            Some(true)
        } else {
            None
        },
        feature_enable_mojang_anti_features: if state.config.features.enable_mojang_anti_features {
            Some(true)
        } else {
            None
        },
        feature_username_check: if state.config.features.username_check {
            Some(true)
        } else {
            None
        },
        links: MetaLinks {
            homepage: state.config.server.base_url.clone(),
            register: state.config.meta.register_url.clone(),
        },
    };

    let response = MetaResponse {
        meta,
        skin_domains: state.config.texture_domains(),
        signature_publickey: pem,
    };

    Ok((
        [(
            HeaderName::from_static("x-authlib-injector-api-location"),
            "/service",
        )],
        JsonResponse(response),
    ))
}
