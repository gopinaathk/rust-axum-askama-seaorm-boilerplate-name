//! `sessions` table, backing store for `tower-sessions`.
//!
//! `data` holds the serialised session map, `expiry_date` is used both to
//! reject stale sessions on load and to purge rows in the cleanup task.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub data: Json,
    pub expiry_date: TimeDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
