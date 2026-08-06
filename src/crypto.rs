//! 密码学工具:RSA 密钥、SHA1withRSA 签名、argon2id 密码哈希、UUID 生成

use anyhow::{Context, Result};
use argon2::password_hash::rand_core::OsRng;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use md5::Md5;
use rand::RngCore;
use rsa::pkcs1v15::{SigningKey, VerifyingKey};
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::path::Path;
use time::OffsetDateTime;
use uuid::Uuid;

/// 生成 RSA-2048 私钥(同时返回公钥)
pub fn generate_keypair() -> Result<(RsaPrivateKey, RsaPublicKey)> {
    let mut rng = OsRng;
    let private = RsaPrivateKey::new(&mut rng, 2048).context("failed to generate RSA key")?;
    let public = RsaPublicKey::from(&private);
    Ok((private, public))
}

/// 加载或生成 RSA 私钥;不存在时生成并写入 PKCS#8 PEM 文件
pub fn load_or_generate_key(path: &Path) -> Result<(RsaPrivateKey, RsaPublicKey)> {
    if path.exists() {
        let pem = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read key file: {}", path.display()))?;
        let private = RsaPrivateKey::from_pkcs8_pem(&pem)
            .with_context(|| format!("invalid private key in {}", path.display()))?;
        let public = RsaPublicKey::from(&private);
        Ok((private, public))
    } else {
        let (private, public) = generate_keypair()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create dir: {}", parent.display()))?;
        }
        let pem = private.to_pkcs8_pem(LineEnding::LF).context("key to PEM failed")?;
        std::fs::write(path, pem.as_bytes())
            .with_context(|| format!("failed to write key file: {}", path.display()))?;
        Ok((private, public))
    }
}

/// SHA1withRSA(PKCS#1 v1.5)签名,与 Java `Signature.getInstance("SHA1withRSA")` 兼容
pub fn sign_sha1(key: &RsaPrivateKey, data: &[u8]) -> Result<Vec<u8>> {
    let signing_key = SigningKey::<Sha1>::new(key.clone());
    let sig = signing_key.sign(data);
    Ok(sig.to_vec())
}

/// 验证 SHA1withRSA 签名
pub fn verify_sha1(key: &RsaPublicKey, data: &[u8], signature: &[u8]) -> bool {
    let Ok(sig) = rsa::pkcs1v15::Signature::try_from(signature) else {
        return false;
    };
    let verifying_key = VerifyingKey::<Sha1>::new(key.clone());
    verifying_key.verify(data, &sig).is_ok()
}

/// 公钥 PEM(用于 meta.signaturePublickey)
pub fn public_key_pem(key: &RsaPublicKey) -> Result<String> {
    Ok(key
        .to_public_key_pem(LineEnding::LF)
        .context("public key to PEM failed")?)
}

/// 计算 SHA-256 十六进制摘要(材质 hash)
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// argon2id 密码哈希,输出 PHC 字符串
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("password hashing failed: {}", e))?
        .to_string())
}

/// 校验密码与 PHC 哈希是否匹配
pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// 离线兼容角色 UUID:`UUID.nameUUIDFromBytes(("OfflinePlayer:" + name).getBytes(UTF-8))`
/// 即 MD5 摘要后设置 version=3、variant=IETF(无符号字符串)
pub fn offline_uuid(player_name: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{}", player_name).as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // Java 实现:version 3, IETF variant
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes).simple().to_string()
}

/// 随机 UUID v4 无符号字符串
pub fn random_uuid() -> String {
    Uuid::new_v4().simple().to_string()
}

/// 随机访问令牌:32 字节随机数的十六进制串
pub fn random_token() -> String {
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    hex::encode(buf)
}

/// 当前 Unix 毫秒时间戳(Java 时间戳格式)
pub fn now_millis() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp() * 1000
}

/// 以毫秒为单位的时间字符串,用于 RFC3339 输出(如 certificates 的 expiresAt)
pub fn millis_to_rfc3339(millis: i64) -> String {
    let secs = millis.div_euclid(1000);
    let nanos = (millis.rem_euclid(1000) * 1_000_000) as i32;
    OffsetDateTime::from_unix_timestamp(secs)
        .map(|dt| dt + time::Duration::nanoseconds(nanos as i64))
        .ok()
        .and_then(|dt| dt.format(&time::format_description::well_known::Rfc3339).ok())
        .unwrap_or_else(String::new)
}

/// Base64 编码
pub fn b64_encode(data: &[u8]) -> String {
    BASE64.encode(data)
}

/// Base64 解码
pub fn b64_decode(data: &str) -> Result<Vec<u8>> {
    Ok(BASE64.decode(data).context("invalid base64")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offline_uuid() {
        // 与 Java UUID.nameUUIDFromBytes("OfflinePlayer:Steve") 一致(Python 交叉验证)
        assert_eq!(
            offline_uuid("Steve"),
            "5627dd98e6be3c21b8a8e92344183641".to_string()
        );
        // 名称不同则 UUID 不同
        assert_ne!(offline_uuid("Steve"), offline_uuid("Alex"));
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let (private, public) = generate_keypair().unwrap();
        let data = b"hello yggr";
        let sig = sign_sha1(&private, data).unwrap();
        assert!(verify_sha1(&public, data, &sig));
        assert!(!verify_sha1(&public, b"tampered", &sig));
    }

    #[test]
    fn test_password_hash() {
        let hash = hash_password("hunter2").unwrap();
        assert!(verify_password("hunter2", &hash));
        assert!(!verify_password("wrong", &hash));
    }

    #[test]
    fn test_sha256() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
