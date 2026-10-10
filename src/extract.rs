use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header, StatusCode};
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
impl<S> FromRequestParts<S> for ClientIp
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        if let Some(info) = parts
            .extensions
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
        {
            return Ok(Self(info.0.ip().to_string()));
        }
        Ok(Self("127.0.0.1".into()))
    }
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
