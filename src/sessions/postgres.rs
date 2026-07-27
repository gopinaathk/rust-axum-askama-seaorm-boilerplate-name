//! Postgres session store, built on SeaORM.
//!
//! Sessions live in the `sessions` table, so they survive restarts and can be
//! revoked server side. Using SeaORM (instead of a `sqlx` store) keeps a single
//! database driver and one connection pool in the dependency tree.

use std::time::Duration;

use async_trait::async_trait;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter,
    sea_query::OnConflict,
};
use time::OffsetDateTime;
use tower_sessions::{
    SessionStore,
    session::{Id, Record},
    session_store,
};

use crate::entities::sessions;

#[derive(Clone, Debug)]
pub struct PostgresSessionStore {
    db: DatabaseConnection,
}

impl PostgresSessionStore {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Removes every expired session row. Returns the number of deleted rows.
    pub async fn delete_expired(&self) -> Result<u64, DbErr> {
        let result = sessions::Entity::delete_many()
            .filter(sessions::Column::ExpiryDate.lt(OffsetDateTime::now_utc()))
            .exec(&self.db)
            .await?;

        Ok(result.rows_affected)
    }

    /// Number of live sessions, for the health page.
    pub async fn count_active(&self) -> Result<u64, DbErr> {
        use sea_orm::PaginatorTrait;

        sessions::Entity::find()
            .filter(sessions::Column::ExpiryDate.gt(OffsetDateTime::now_utc()))
            .count(&self.db)
            .await
    }

    /// Background task that purges expired sessions on a fixed interval.
    pub fn spawn_cleanup_task(self, period: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(period);
            // The first tick fires immediately; skip it so boot stays quiet.
            ticker.tick().await;

            loop {
                ticker.tick().await;

                match self.delete_expired().await {
                    Ok(0) => {}
                    Ok(deleted) => tracing::debug!(deleted, "purged expired sessions"),
                    Err(error) => tracing::error!(%error, "session cleanup failed"),
                }
            }
        })
    }
}

#[async_trait]
impl SessionStore for PostgresSessionStore {
    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let data = serde_json::to_value(&record.data)
            .map_err(|error| session_store::Error::Encode(error.to_string()))?;

        let model = sessions::ActiveModel {
            id: Set(record.id.to_string()),
            data: Set(data),
            expiry_date: Set(record.expiry_date),
        };

        sessions::Entity::insert(model)
            .on_conflict(
                OnConflict::column(sessions::Column::Id)
                    .update_columns([sessions::Column::Data, sessions::Column::ExpiryDate])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(backend_error)?;

        Ok(())
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let found = sessions::Entity::find_by_id(session_id.to_string())
            .one(&self.db)
            .await
            .map_err(backend_error)?;

        let Some(model) = found else {
            return Ok(None);
        };

        if model.expiry_date <= OffsetDateTime::now_utc() {
            self.delete(session_id).await?;
            return Ok(None);
        }

        let data = serde_json::from_value(model.data)
            .map_err(|error| session_store::Error::Decode(error.to_string()))?;

        Ok(Some(Record {
            id: *session_id,
            data,
            expiry_date: model.expiry_date,
        }))
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        sessions::Entity::delete_by_id(session_id.to_string())
            .exec(&self.db)
            .await
            .map_err(backend_error)?;

        Ok(())
    }
}

fn backend_error(error: DbErr) -> session_store::Error {
    session_store::Error::Backend(error.to_string())
}
