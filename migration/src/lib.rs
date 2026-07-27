//! Database migrations for the application.
//!
//! Every migration lives in its own module and is registered in [`Migrator::migrations`].
//! Run them from the CLI with `cargo run -p migration -- up`, or let the web server apply
//! them on boot (`RUN_MIGRATIONS=true`, the default).

pub use sea_orm_migration::prelude::*;

mod m20260101_000001_create_users_table;
mod m20260101_000002_create_sessions_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260101_000001_create_users_table::Migration),
            Box::new(m20260101_000002_create_sessions_table::Migration),
        ]
    }
}
