use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;

use crate::error::AppError;
use crate::extract::CurrentUser;
use crate::rbac::{AuthHit, authorize, Perm};
use crate::state::AppState;

use super::audit_user;

/// 超管应用签名：签名材料只活在数据库（`app_signing` 单例行），空库时点「签名发布」
/// 自动生成一把（`keytool -genkeypair`，release 级 RSA），超管也可随时上传/覆盖自己的；
/// 正式签名在服务端完成（镜像自带 JRE + apksigner），签后再算 sha256 写 version.json。
/// 密码与 keystore 永不进日志、永不出现在 API 响应里（只回显别名与 keystore 指纹），
/// 且不提供任何导出/下载路由（备份靠数据库密文，那是运维路径）。
fn keystore_fingerprint(keystore: &[u8]) -> String {
    hex::encode(Sha256::digest(keystore))
}

/// 用完即删的临时文件（签名材料落盘中转，重命名/失败都清理）。
struct TempFile(PathBuf);

impl TempFile {
    fn create(prefix: &str, bytes: &[u8]) -> Result<Self, AppError> {
        let mut path = std::env::temp_dir();
        path.push(format!("{prefix}-{}", crate::crypto::random_token()));
        std::fs::write(&path, bytes).map_err(|_| AppError::internal())?;
        #[cfg(unix)]
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o600))
            .map_err(|_| AppError::internal())?;
        Ok(Self(path))
    }

    fn path_for(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("{prefix}-{}", crate::crypto::random_token()));
        path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_file()
        && path
            .metadata()
            .map(|item| item.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn apksigner_path() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "apksigner.bat"
    } else {
        "apksigner"
    };
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|item| is_executable(item))
    })
}

struct SigningRow {
    keystore: Vec<u8>,
    keystore_password: Vec<u8>,
    key_alias: String,
    key_password: Vec<u8>,
}

async fn load_row(pool: &sqlx::PgPool, local_key: &[u8; 32]) -> Result<Option<SigningRow>, AppError> {
    let row = sqlx::query(
        r#"SELECT keystore_ciphertext, keystore_password_ciphertext, key_alias, key_password_ciphertext
           FROM app_signing WHERE id = 1"#,
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let decrypt = |value: Vec<u8>| crate::crypto::decrypt(local_key, &value);
    Ok(Some(SigningRow {
        keystore: decrypt(row.get::<Vec<u8>, _>("keystore_ciphertext")).map_err(|_| AppError::internal())?,
        keystore_password: decrypt(row.get::<Vec<u8>, _>("keystore_password_ciphertext"))
            .map_err(|_| AppError::internal())?,
        key_alias: row.get::<String, _>("key_alias"),
        key_password: decrypt(row.get::<Vec<u8>, _>("key_password_ciphertext"))
            .map_err(|_| AppError::internal())?,
    }))
}

/// 空库自动生成的固定别名（换 key 请走 PUT 覆盖上传）。
const GENERATED_ALIAS: &str = "duolinzao";

fn keytool_path() -> Option<PathBuf> {
    let name = if cfg!(windows) { "keytool.exe" } else { "keytool" };
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|item| is_executable(item))
    })
}

async fn row_exists(pool: &sqlx::PgPool) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM app_signing WHERE id = 1)")
        .fetch_one(pool)
        .await?;
    Ok(exists)
}

async fn save_row(
    pool: &sqlx::PgPool,
    local_key: &[u8; 32],
    alias: &str,
    keystore: &[u8],
    keystore_password: &str,
    key_password: &str,
) -> Result<(), AppError> {
    let encrypt = |data: &[u8]| crate::crypto::encrypt(local_key, data);
    sqlx::query(
        r#"INSERT INTO app_signing (id, keystore_ciphertext, keystore_password_ciphertext, key_alias, key_password_ciphertext, updated_at)
           VALUES (1, $1, $2, $3, $4, now())
           ON CONFLICT (id) DO UPDATE SET keystore_ciphertext = $1, keystore_password_ciphertext = $2,
               key_alias = $3, key_password_ciphertext = $4, updated_at = now()"#,
    )
    .bind(encrypt(keystore)?)
    .bind(encrypt(keystore_password.as_bytes())?)
    .bind(alias)
    .bind(encrypt(key_password.as_bytes())?)
    .execute(pool)
    .await?;
    Ok(())
}

/// 空库时 `keytool -genkeypair` 生成一把 release 级 RSA keystore 并加密入库。
/// 已有行时调用方应先拦（`generate_keystore`），并发竞态下后写覆盖、先写审计为准。
/// 密码只走进程环境进 keytool（argv/`ps` 不可见），永不进日志与审计。
async fn generate_and_store(
    state: &AppState,
    user: &CurrentUser,
    hit: &AuthHit,
) -> Result<(String, String), AppError> {
    let Some(keytool) = keytool_path() else {
        return Err(AppError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "signing_unavailable",
            "服务端缺少 keytool（镜像应自带 JRE）",
        ));
    };
    let ks_path = TempFile::path_for("dlz-gen-ks");
    let _guard = TempFile(ks_path.clone());
    let ks_password = crate::crypto::random_token();
    let key_password = crate::crypto::random_token();
    let alias = GENERATED_ALIAS.to_string();
    let generated = Command::new(&keytool)
        .arg("-genkeypair")
        .arg("-v")
        .arg("-keystore")
        .arg(&ks_path)
        .arg("-storetype")
        .arg("JKS")
        .arg("-keyalg")
        .arg("RSA")
        .arg("-keysize")
        .arg("2048")
        .arg("-validity")
        .arg("10000")
        .arg("-alias")
        .arg(&alias)
        .arg("-storepass:env")
        .arg("DLZ_GEN_KS_PASS")
        .arg("-keypass:env")
        .arg("DLZ_GEN_KEY_PASS")
        .arg("-dname")
        .arg("CN=Duolinzao, OU=Release, O=Duolinzao, L=Unknown, ST=Unknown, C=CN")
        .env("DLZ_GEN_KS_PASS", &ks_password)
        .env("DLZ_GEN_KEY_PASS", &key_password)
        .output()
        .map_err(|_| AppError::internal())?;
    if !generated.status.success() {
        return Err(AppError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "signing_generate_failed",
            "keystore 自动生成失败",
        ));
    }
    let keystore = std::fs::read(&ks_path).map_err(|_| AppError::internal())?;
    if keystore.is_empty() {
        return Err(AppError::internal());
    }
    save_row(
        &state.pool,
        &state.config.local_key,
        &alias,
        &keystore,
        &ks_password,
        &key_password,
    )
    .await?;
    let fingerprint = keystore_fingerprint(&keystore);
    audit_user(
        state,
        user,
        hit,
        "app_signing",
        Some("app_signing"),
        None,
        // 只记别名 + keystore 指纹：密码与 keystore 本体永不进审计。
        // 注意审计 summary 会过 sanitize（子串 "key"/"code" 会被整键剥掉），
        // 所以这里用 alias/fingerprint，绝不用 key_alias/keystore_sha256。
        json!({"alias": alias, "fingerprint": fingerprint, "generated": true}),
    )
    .await?;
    Ok((alias, fingerprint))
}

/// 仅空库时生成并返回状态；已配置则 400（请走 PUT 覆盖上传）。
pub async fn generate_keystore(state: &AppState, user: &CurrentUser) -> Result<Value, AppError> {
    let hit = authorize(&user.assignments, Perm::SystemSuper, None)?;
    if row_exists(&state.pool).await? {
        return Err(AppError::bad(
            "signing_already_configured",
            "已配置签名 keystore，如需更换请在同页上传覆盖",
        ));
    }
    let (alias, fingerprint) = generate_and_store(state, user, &hit).await?;
    Ok(status_body(true, Some(&alias), Some(&fingerprint), None))
}

fn status_body(configured: bool, alias: Option<&str>, fingerprint: Option<&str>, updated: Option<&str>) -> Value {
    json!({
        "configured": configured,
        "key_alias": alias,
        "keystore_sha256": fingerprint,
        "updated_at": updated,
    })
}

pub async fn status(state: &AppState, user: &CurrentUser) -> Result<Value, AppError> {
    let _ = authorize(&user.assignments, Perm::SystemSuper, None)?;
    let row = sqlx::query("SELECT key_alias, updated_at FROM app_signing WHERE id = 1")
        .fetch_optional(&state.pool)
        .await?;
    let Some(row) = row else {
        return Ok(status_body(false, None, None, None));
    };
    // 指纹要读 keystore 本体：只在已配置时多查一次，不把本体带出本函数。
    let fingerprint: Option<String> =
        sqlx::query_scalar("SELECT keystore_ciphertext FROM app_signing WHERE id = 1")
            .fetch_optional(&state.pool)
            .await?
            .map(|cipher: Vec<u8>| {
                crate::crypto::decrypt(&state.config.local_key, &cipher)
                    .map(|raw| keystore_fingerprint(&raw))
                    .unwrap_or_default()
            });
    Ok(status_body(
        true,
        Some(row.get::<String, _>("key_alias").as_str()),
        fingerprint.as_deref(),
        Some(
            row.get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                .to_rfc3339()
                .as_str(),
        ),
    ))
}

pub async fn put_keystore(
    state: &AppState,
    user: &CurrentUser,
    keystore: &[u8],
    keystore_password: &str,
    key_alias: &str,
    key_password: &str,
) -> Result<Value, AppError> {
    let hit = authorize(&user.assignments, Perm::SystemSuper, None)?;
    if keystore.is_empty() || keystore.len() > 1_048_576 {
        return Err(AppError::bad("signing_keystore_invalid", "keystore 文件为空或过大（上限 1MB）"));
    }
    if keystore_password.is_empty() || key_alias.trim().is_empty() || key_password.is_empty() {
        return Err(AppError::bad(
            "signing_field_required",
            "需要 keystore 密码、别名、key 密码",
        ));
    }
    let alias = key_alias.trim().to_string();
    let existed = row_exists(&state.pool).await?;
    save_row(
        &state.pool,
        &state.config.local_key,
        &alias,
        keystore,
        keystore_password,
        key_password,
    )
    .await?;
    let fingerprint = keystore_fingerprint(keystore);
    audit_user(
        state,
        user,
        &hit,
        "app_signing",
        Some("app_signing"),
        None,
        // 只记别名 + keystore 指纹 + 字节数 + 是否覆盖：密码与 keystore 本体永不进审计。
        // 字段名避开 sanitize 子串（"key"/"code" 整键剥掉）：不用 key_alias/keystore_sha256/keystore_bytes。
        json!({"alias": alias, "fingerprint": fingerprint, "file_bytes": keystore.len(), "replaced": existed}),
    )
    .await?;
    Ok(status_body(true, Some(&alias), Some(&fingerprint), None))
}

pub async fn sign_release(state: &AppState, user: &CurrentUser) -> Result<Value, AppError> {
    let hit = authorize(&user.assignments, Perm::SystemSuper, None)?;
    // 空库时先自动生成一把入库再签（已有则不动）；响应带 keystore_generated 供 UI 提示，
    // 审计记 auto_generated（sanitize 会剥掉含 "key" 子串的键）。
    let (signing, generated) = match load_row(&state.pool, &state.config.local_key).await? {
        Some(row) => (row, false),
        None => {
            generate_and_store(state, user, &hit).await?;
            let row = load_row(&state.pool, &state.config.local_key)
                .await?
                .ok_or_else(AppError::internal)?;
            (row, true)
        }
    };
    let Some(dir) = state.config.resolved_app_dir() else {
        return Err(AppError::bad("app_dir_missing", "服务端未配置静态目录"));
    };
    let Some(signer) = apksigner_path() else {
        return Err(AppError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "signing_unavailable",
            "服务端缺少 apksigner（镜像应自带 JRE + build-tools）",
        ));
    };
    // 待签名的是 CI 打进镜像的未签包；版本号沿用它的 version.json，只替换摘要与标注。
    let staged_meta: Value = std::fs::read_to_string(format!("{dir}/version.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(|body: &Value| body.get("versionCode").and_then(Value::as_i64).unwrap_or(0) > 0)
        .ok_or_else(|| AppError::bad("release_missing", "待签名的 version.json 不存在或非法"))?;
    let staged_apk = format!("{dir}/duolinzao.apk");
    if !Path::new(&staged_apk).is_file() {
        return Err(AppError::bad("release_missing", "待签名的 duolinzao.apk 不存在"));
    }
    let alias = signing.key_alias.clone();
    let ks_file = TempFile::create(
        "dlz-sign-ks",
        &signing.keystore,
    )?;
    let out_path = TempFile::path_for("dlz-sign-out");
    let _out_guard = TempFile(out_path.clone());
    let ks_password =
        String::from_utf8(signing.keystore_password.clone()).map_err(|_| AppError::internal())?;
    let key_password =
        String::from_utf8(signing.key_password.clone()).map_err(|_| AppError::internal())?;
    let signed = Command::new(&signer)
        .arg("sign")
        .arg("--ks")
        .arg(&ks_file.0)
        .arg("--ks-pass")
        .arg("env:DLZ_SIGN_KS_PASS")
        .arg("--ks-key-alias")
        .arg(&alias)
        .arg("--key-pass")
        .arg("env:DLZ_SIGN_KEY_PASS")
        .arg("--out")
        .arg(&out_path)
        .arg(&staged_apk)
        .env("DLZ_SIGN_KS_PASS", &ks_password)
        .env("DLZ_SIGN_KEY_PASS", &key_password)
        .output()
        .map_err(|_| AppError::internal())?;
    if !signed.status.success() {
        return Err(AppError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "sign_failed",
            "apksigner 签名失败（keystore/密码/别名可能不对）",
        ));
    }
    let verified = Command::new(&signer)
        .arg("verify")
        .arg(&out_path)
        .output()
        .map_err(|_| AppError::internal())?;
    if !verified.status.success() {
        return Err(AppError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "sign_failed",
            "签名后校验未通过，已丢弃",
        ));
    }
    let bytes = std::fs::read(&out_path).map_err(|_| AppError::internal())?;
    let sha256 = hex::encode(Sha256::digest(&bytes));
    std::fs::rename(&out_path, &staged_apk).map_err(|_| AppError::internal())?;
    let version_code = staged_meta.get("versionCode").cloned().unwrap_or(json!(0));
    let version_name = staged_meta
        .get("versionName")
        .and_then(Value::as_str)
        .unwrap_or("");
    let url = staged_meta.get("url").and_then(Value::as_str).unwrap_or("/app/duolinzao.apk");
    std::fs::write(
        format!("{dir}/version.json"),
        format!(
            "{}\n",
            json!({
                "versionCode": version_code,
                "versionName": version_name,
                "url": url,
                "sha256": sha256,
                "signing": "release",
            })
        ),
    )
    .map_err(|_| AppError::internal())?;
    let fingerprint = keystore_fingerprint(&signing.keystore);
    audit_user(
        state,
        user,
        &hit,
        "app_sign_release",
        Some("app_signing"),
        None,
        json!({
            "alias": alias,
            "fingerprint": fingerprint,
            "version": version_code,
            "apk_sha256": sha256,
            "signing": "release",
            "auto_generated": generated,
        }),
    )
    .await?;
    Ok(json!({
        "versionCode": version_code,
        "versionName": version_name,
        "url": url,
        "sha256": sha256,
        "signing": "release",
        "keystore_generated": generated,
    }))
}
