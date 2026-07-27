//! Queries and writes for the `users` table.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter,
};

use crate::entities::users;

#[derive(Clone, Debug)]
pub struct UserRepository {
    db: DatabaseConnection,
}

impl UserRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<users::Model>, DbErr> {
        users::Entity::find_by_id(id).one(&self.db).await
    }

    /// Emails are stored normalised (trimmed + lowercased) by `create`.
    pub async fn find_by_email(&self, email: &str) -> Result<Option<users::Model>, DbErr> {
        users::Entity::find()
            .filter(users::Column::Email.eq(normalize_email(email)))
            .one(&self.db)
            .await
    }

    pub async fn email_taken(&self, email: &str) -> Result<bool, DbErr> {
        let count = users::Entity::find()
            .filter(users::Column::Email.eq(normalize_email(email)))
            .count(&self.db)
            .await?;

        Ok(count > 0)
    }

    pub async fn create(
        &self,
        name: &str,
        email: &str,
        password_hash: String,
    ) -> Result<users::Model, DbErr> {
        let now = Utc::now().into();

        users::ActiveModel {
            name: Set(name.trim().to_owned()),
            email: Set(normalize_email(email)),
            password_hash: Set(password_hash),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&self.db)
        .await
    }

    pub async fn count(&self) -> Result<u64, DbErr> {
        users::Entity::find().count(&self.db).await
    }
}

/// Case-insensitive, whitespace-tolerant email key.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::normalize_email;

    #[test]
    fn normalizes_email() {
        assert_eq!(normalize_email("  Ada@Example.COM "), "ada@example.com");
    }
}
