use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use sqlx::Row;
use std::net::SocketAddr;
use uuid::Uuid;

use crate::error::AppError;
use crate::rbac::Assignment;
use crate::state::AppState;

pub struct ClientIp(pub String);

#[async_trait]
impl FromRequestParts<AppState> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let fallback = parts
            .extensions
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
            .map(|info| info.0.ip().to_string())
            .unwrap_or_else(|| "127.0.0.1".into());
        if !state.config.trust_proxy {
            return Ok(Self(fallback));
        }
        // 只信任一层可信反代：取 X-Forwarded-For 首段，否则取 Forwarded 首个 for=。
        // 开关关闭时绝不读这些头，防止客户端伪造 IP 绕过限流/审计。
        if let Some(ip) = forwarded_client_ip(&parts.headers) {
            return Ok(Self(ip));
        }
        Ok(Self(fallback))
    }
}

/// 从代理头里取一层客户端 IP。只取首段，清洗引号/括号/端口，非法字符直接丢弃。
pub fn forwarded_client_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(raw) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        let first = raw.split(',').next().unwrap_or("").trim();
        if let Some(ip) = clean_ip_token(first) {
            return Some(ip);
        }
    }
    if let Some(raw) = headers.get("forwarded").and_then(|v| v.to_str().ok()) {
        // Forwarded: for=1.2.3.4;proto=https, for="[2001:db8::1]:1234"
        let first = raw.split(',').next().unwrap_or("");
        for part in first.split(';') {
            let part = part.trim();
            if let Some(value) = part
                .strip_prefix("for=")
                .or_else(|| part.strip_prefix("For="))
                .or_else(|| part.strip_prefix("FOR="))
            {
                if let Some(ip) = clean_ip_token(value.trim()) {
                    return Some(ip);
                }
            }
        }
    }
    None
}

fn clean_ip_token(raw: &str) -> Option<String> {
    let mut token = raw.trim().trim_matches('"').trim().to_string();
    if token.is_empty() || token.len() > 256 {
        return None;
    }
    // [v6]:port 或 [v6] 形式先去括号。
    if let Some(rest) = token.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            token = rest[..end].to_string();
        }
    } else if token.matches(':').count() == 1 && !token.contains(' ') {
        // 疑似 host:port（单冒号），剥掉端口；多冒号按 IPv6 原样保留。
        if let Some((host, _)) = token.rsplit_once(':') {
            if !host.is_empty() && !host.contains(':') {
                token = host.to_string();
            }
        }
    }
    token = token.trim().trim_matches('"').to_string();
    if token.is_empty() || token.len() > 256 {
        return None;
    }
    if !token
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '-' | '_' ))
    {
        return None;
    }
    Some(token)
}

pub struct UserAgent(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for UserAgent
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        Ok(Self(value))
    }
}

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub account_id: Uuid,
    pub username: String,
    pub session_id: Uuid,
    pub device_id: Uuid,
    pub assignments: Vec<Assignment>,
}

#[async_trait]
impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get("dlz_session")
            .map(|cookie| cookie.value().to_string())
            .ok_or_else(AppError::unauthorized)?;
        load_session(state, &token, state.clock.now()).await
    }
}

pub async fn load_session(
    state: &AppState,
    token: &str,
    now: DateTime<Utc>,
) -> Result<CurrentUser, AppError> {
    let hash = crate::crypto::sha256_hex(token);
    let row = sqlx::query(
        r#"
        SELECT s.id AS session_id, s.device_id, s.account_id, a.username
        FROM session s
        JOIN account a ON a.id = s.account_id
        WHERE s.token_hash = $1
          AND s.revoked_at IS NULL
          AND s.expires_at > $2
        "#,
    )
    .bind(hash)
    .bind(now)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(AppError::unauthorized)?;
    let account_id: Uuid = row.get("account_id");
    let assignments = load_assignments(&state.pool, account_id).await?;
    Ok(CurrentUser {
        account_id,
        username: row.get("username"),
        session_id: row.get("session_id"),
        device_id: row.get("device_id"),
        assignments,
    })
}

pub async fn load_assignments(
    pool: &sqlx::PgPool,
    account_id: Uuid,
) -> Result<Vec<Assignment>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT role, canteen_ids, source_label
        FROM role_assignment
        WHERE account_id = $1
        ORDER BY created_at
        "#,
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| Assignment {
            role: row.get("role"),
            canteen_ids: row.get("canteen_ids"),
            source_label: row.get("source_label"),
        })
        .collect())
}

pub fn rejection_status(err: &AppError) -> StatusCode {
    err.status
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in pairs {
            map.insert(
                k.parse::<axum::http::HeaderName>().unwrap(),
                v.parse().unwrap(),
            );
        }
        map
    }

    #[test]
    fn xff_takes_first_segment_only() {
        let map = headers(&[("x-forwarded-for", "1.2.3.4, 10.0.0.1, 192.168.1.1")]);
        assert_eq!(forwarded_client_ip(&map).as_deref(), Some("1.2.3.4"));
    }

    #[test]
    fn forwarded_takes_first_for() {
        let map = headers(&[("forwarded", "for=9.9.9.9;proto=https, for=10.0.0.2")]);
        assert_eq!(forwarded_client_ip(&map).as_deref(), Some("9.9.9.9"));
    }

    #[test]
    fn rejects_injected_tokens() {
        let map = headers(&[("x-forwarded-for", "1.2.3.4; rm -rf")]);
        assert_eq!(forwarded_client_ip(&map), None);
        let map = headers(&[("forwarded", "for=\"[2001:db8::1]:1234\"")]);
        assert_eq!(
            forwarded_client_ip(&map).as_deref(),
            Some("2001:db8::1")
        );
    }
}
