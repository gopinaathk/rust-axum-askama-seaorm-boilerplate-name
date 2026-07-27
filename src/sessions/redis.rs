//! Redis session store.
//!
//! Each session is a single JSON string keyed by `<prefix><session-id>`, with
//! the record's expiry mapped onto the key's TTL. Redis evicts stale sessions
//! itself, so no cleanup task is needed.

use async_trait::async_trait;
use redis::{AsyncCommands, RedisError, aio::ConnectionManager};
use time::OffsetDateTime;
use tower_sessions::{
    SessionStore,
    session::{Id, Record},
    session_store,
};

#[derive(Clone)]
pub struct RedisSessionStore {
    connection: ConnectionManager,
    key_prefix: String,
}

impl std::fmt::Debug for RedisSessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisSessionStore")
            .field("key_prefix", &self.key_prefix)
            .finish_non_exhaustive()
    }
}

impl RedisSessionStore {
    /// Opens a multiplexed, auto-reconnecting connection.
    pub async fn connect(url: &str, key_prefix: &str) -> Result<Self, RedisError> {
        let client = redis::Client::open(url.to_owned())?;
        let connection = ConnectionManager::new(client).await?;

        Ok(Self {
            connection,
            key_prefix: key_prefix.to_owned(),
        })
    }

    fn key(&self, session_id: &Id) -> String {
        format!("{}{}", self.key_prefix, session_id)
    }

    /// `PING` round-trip, used by the health page.
    pub async fn ping(&self) -> Result<(), RedisError> {
        let mut connection = self.connection.clone();
        redis::cmd("PING")
            .query_async::<String>(&mut connection)
            .await?;

        Ok(())
    }

    /// Number of session keys currently stored (`SCAN` over the prefix).
    pub async fn count_active(&self) -> Result<u64, RedisError> {
        let mut connection = self.connection.clone();
        let mut cursor: u64 = 0;
        let mut total: u64 = 0;

        loop {
            let (next, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(format!("{}*", self.key_prefix))
                .arg("COUNT")
                .arg(500)
                .query_async(&mut connection)
                .await?;

            total += keys.len() as u64;
            cursor = next;

            if cursor == 0 {
                break;
            }
        }

        Ok(total)
    }
}

#[async_trait]
impl SessionStore for RedisSessionStore {
    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let payload = serde_json::to_string(record)
            .map_err(|error| session_store::Error::Encode(error.to_string()))?;

        // Records already expired are simply dropped: Redis rejects TTL <= 0.
        let ttl = (record.expiry_date - OffsetDateTime::now_utc()).whole_seconds();
        if ttl <= 0 {
            return self.delete(&record.id).await;
        }

        let mut connection = self.connection.clone();
        connection
            .set_ex::<_, _, ()>(self.key(&record.id), payload, ttl as u64)
            .await
            .map_err(backend_error)?;

        Ok(())
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let mut connection = self.connection.clone();
        let payload: Option<String> = connection
            .get(self.key(session_id))
            .await
            .map_err(backend_error)?;

        let Some(payload) = payload else {
            return Ok(None);
        };

        let record: Record = serde_json::from_str(&payload)
            .map_err(|error| session_store::Error::Decode(error.to_string()))?;

        if record.expiry_date <= OffsetDateTime::now_utc() {
            self.delete(session_id).await?;
            return Ok(None);
        }

        Ok(Some(record))
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        let mut connection = self.connection.clone();
        connection
            .del::<_, ()>(self.key(session_id))
            .await
            .map_err(backend_error)?;

        Ok(())
    }
}

fn backend_error(error: RedisError) -> session_store::Error {
    session_store::Error::Backend(error.to_string())
}
