use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::domain::sanitize_summary;
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub at: DateTime<Utc>,
    pub actor_id: Option<Uuid>,
    pub actor_username: Option<String>,
    pub role: Option<String>,
    pub scope: String,
    pub action: String,
    pub object_type: Option<String>,
    pub object_id: Option<String>,
    pub summary: Value,
    pub result: String,
}

pub async fn write_audit<'e, E>(executor: E, entry: AuditEntry) -> Result<(), AppError>
where
    E: PgExecutor<'e>,
{
    let summary = sanitize_summary(entry.summary);
    sqlx::query(
        r#"
        INSERT INTO audit_log (
            created_at, actor_id, actor_username, role, scope, action,
            object_type, object_id, summary, result
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(entry.at)
    .bind(entry.actor_id)
    .bind(entry.actor_username)
    .bind(entry.role)
    .bind(entry.scope)
    .bind(entry.action)
    .bind(entry.object_type)
    .bind(entry.object_id)
    .bind(summary)
    .bind(entry.result)
    .execute(executor)
    .await?;
    Ok(())
}
