//! Application library crate.
//!
//! Layering (outermost first):
//!
//! * `web`       – HTTP: router, extractors, middleware, handlers, templates
//! * `services`  – use cases / business rules (registration, login, ...)
//! * `repositories` – data access, the only place that talks to SeaORM entities
//! * `entities`  – SeaORM models (schema mirror)
//! * `db`        – connection bootstrap, database creation, migrations
//!
//! Handlers never touch entities directly; they call a service, which calls a
//! repository. That keeps the HTTP layer thin and the domain testable.

pub mod config;
pub mod db;
pub mod entities;
pub mod error;
pub mod repositories;
pub mod security;
pub mod services;
pub mod sessions;
pub mod state;
pub mod web;

pub use config::{Config, Environment};
pub use error::{AppError, AppResult};
pub use sessions::AppSessionStore;
pub use state::AppState;
