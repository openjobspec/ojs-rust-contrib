use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

diesel::table! {
    ojs_outbox (id) {
        id -> Uuid,
        job_type -> Text,
        args -> Jsonb,
        queue -> Nullable<Text>,
        priority -> Integer,
        status -> Text,
        created_at -> Timestamptz,
        published_at -> Nullable<Timestamptz>,
        error_message -> Nullable<Text>,
        retry_count -> Integer,
        last_error_at -> Nullable<Timestamptz>,
    }
}

/// Represents the `ojs_outbox` database table.
pub use ojs_outbox as OjsOutbox;

/// A row in the `ojs_outbox` table.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = ojs_outbox)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OutboxEntry {
    pub id: Uuid,
    pub job_type: String,
    pub args: serde_json::Value,
    pub queue: Option<String>,
    pub priority: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub last_error_at: Option<DateTime<Utc>>,
}

/// New outbox entry for insertion.
#[derive(Debug, Insertable)]
#[diesel(table_name = ojs_outbox)]
pub struct NewOutboxEntry {
    pub id: Uuid,
    pub job_type: String,
    pub args: serde_json::Value,
    pub queue: Option<String>,
    pub priority: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
}
