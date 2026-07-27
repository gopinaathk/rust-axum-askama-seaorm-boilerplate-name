//! Data access layer. The only place that speaks SeaORM.

pub mod user_repository;

pub use user_repository::UserRepository;
