//! Migration CLI: `cargo run -p migration -- up | down | status | fresh`.
//!
//! `DATABASE_URL` is used when present. Otherwise it is assembled from the same
//! `DB_*` variables the web server reads, so `.env` stays the single source of
//! truth.

use std::env;

use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    if env::var("DATABASE_URL")
        .map(|url| url.trim().is_empty())
        .unwrap_or(true)
    {
        // SAFETY: single-threaded startup, before any task is spawned.
        unsafe { env::set_var("DATABASE_URL", database_url_from_parts()) };
    }

    cli::run_cli(migration::Migrator).await;
}

fn database_url_from_parts() -> String {
    let host = var("DB_HOST", "localhost");
    let port = var("DB_PORT", "5432");
    let username = var("DB_USERNAME", "postgres");
    let password = var("DB_PASSWORD", "");
    let name = var("DB_NAME", "rust-axum-askama");
    let options = var("DB_OPTIONS", "");

    let credentials = if password.is_empty() {
        encode(&username)
    } else {
        format!("{}:{}", encode(&username), encode(&password))
    };

    let query = if options.is_empty() {
        String::new()
    } else {
        format!("?{}", options.trim_start_matches('?'))
    };

    format!(
        "postgres://{credentials}@{host}:{port}/{}{query}",
        encode(&name)
    )
}

fn var(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

/// Percent-encodes characters that would break the connection URL.
fn encode(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => c.to_string(),
            other => other
                .to_string()
                .as_bytes()
                .iter()
                .map(|byte| format!("%{byte:02X}"))
                .collect(),
        })
        .collect()
}
