//! /service/minecraftservices/player/certificates - Minecraft 1.19+ 消息签名密钥对
//!
//! 响应中的 keyPair 由服务端私钥签名:
//! - publicKeySignature: 对 publicKey(SPKI DER)的 SHA1withRSA 签名
//! - publicKeySignatureV2: 对 {"expiresAt","keyPair"} JSON 的 SHA256withRSA 签名

use axum::extract::State;
use axum::http::HeaderMap;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rand_core::OsRng;
use rsa::pkcs1v15::{Signature, SigningKey};
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
use rsa::signature::{SignatureEncoding, Signer};
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde::Serialize;
use sha1::Sha1;
use sha2::Sha256;

use crate::api::auth::bearer_token;
use crate::app::state::AppState;
use crate::core::crypto::{millis_to_rfc3339, now_millis};
use crate::core::error::{ApiError, ApiResult};
use crate::core::types::JsonResponse;
use tracing::{info, instrument, warn};

/// 密钥对有效期(天)
const KEY_VALIDITY_DAYS: i64 = 7;
/// 建议刷新时间(天)
const KEY_REFRESH_DAYS: i64 = 1;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPairResponse {
    pub private_key: String,
    pub public_key: String,
    pub public_key_signature: String,
    pub public_key_signature_v2: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificatesResponse {
    pub key_pair: KeyPairResponse,
    pub expires_at: String,
    pub refreshed_after: String,
}

fn internal(e: impl std::fmt::Display) -> ApiError {
    tracing::error!(error = %e, "internal error in certificates");
    ApiError::internal(e.to_string())
}

/// POST /service/minecraftservices/player/certificates
#[instrument(skip_all, level = "debug")]
pub async fn certificates(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<JsonResponse<CertificatesResponse>> {
    let tok = bearer_token(&state, &headers).await?;
    if tok.player_id.is_none() {
        warn!(user_id = %tok.user_id, "certificates: no profile assigned");
        return Err(ApiError::bad_request(
            "Access token has no profile assigned.",
        ));
    }

    // 生成短期密钥对
    let (private, public) = generate_session_keypair()?;
    let private_der = private
        .to_pkcs8_der()
        .map_err(|e| internal(format!("key encoding failed: {e}")))?
        .as_bytes()
        .to_vec();
    let public_der = public
        .to_public_key_der()
        .map_err(|e| internal(format!("key encoding failed: {e}")))?
        .as_bytes()
        .to_vec();

    // V1 签名:SHA1withRSA(publicKey DER)
    let sig_v1 = sign_sha1(&state.private_key, &public_der)?;
    // V2 签名:SHA256withRSA({"expiresAt","keyPair"} JSON)
    let now = now_millis();
    let expires_at = now + KEY_VALIDITY_DAYS * 24 * 3600 * 1000;
    let expires_at_str = millis_to_rfc3339(expires_at);
    let private_b64 = BASE64.encode(&private_der);
    let public_b64 = BASE64.encode(&public_der);
    let v2_payload = format!(
        r#"{{"expiresAt":"{}","keyPair":{{"privateKey":"{}","publicKey":"{}"}}}}"#,
        expires_at_str, private_b64, public_b64
    );
    let sig_v2 = sign_sha256(&state.private_key, v2_payload.as_bytes())?;

    info!(user_id = %tok.user_id, "certificates issued");
    Ok(JsonResponse(CertificatesResponse {
        key_pair: KeyPairResponse {
            private_key: private_b64,
            public_key: public_b64,
            public_key_signature: BASE64.encode(&sig_v1),
            public_key_signature_v2: BASE64.encode(&sig_v2),
        },
        expires_at: expires_at_str,
        refreshed_after: millis_to_rfc3339(now + KEY_REFRESH_DAYS * 24 * 3600 * 1000),
    }))
}

/// 生成会话 RSA-2048 密钥对
fn generate_session_keypair() -> Result<(RsaPrivateKey, RsaPublicKey), ApiError> {
    let mut rng = OsRng;
    let private =
        RsaPrivateKey::new(&mut rng, 2048).map_err(|e| internal(format!("keygen failed: {e}")))?;
    let public = RsaPublicKey::from(&private);
    Ok((private, public))
}

/// SHA1withRSA 签名
fn sign_sha1(key: &rsa::RsaPrivateKey, data: &[u8]) -> Result<Vec<u8>, ApiError> {
    let signing_key = SigningKey::<Sha1>::new(key.clone());
    let sig: Signature = signing_key.sign(data);
    Ok(sig.to_vec())
}

/// SHA256withRSA 签名
fn sign_sha256(key: &rsa::RsaPrivateKey, data: &[u8]) -> Result<Vec<u8>, ApiError> {
    let signing_key = SigningKey::<Sha256>::new(key.clone());
    let sig: Signature = signing_key.sign(data);
    Ok(sig.to_vec())
}
