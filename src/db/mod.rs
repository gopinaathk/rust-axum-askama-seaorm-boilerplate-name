//! Database bootstrap: create the database if it is missing, connect, migrate.

use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement,
    Value,
};

use crate::config::DatabaseConfig;

/// Connects to the application database, creating and migrating it when needed.
pub async fn connect(config: &DatabaseConfig) -> Result<DatabaseConnection, DbErr> {
    tracing::debug!(
        endpoint = %config.endpoint(),
        database = %config.database_name(),
        user = %config.username,
        password_set = !config.password.is_empty(),
        "resolving database connection"
    );

    if config.auto_create {
        ensure_database_exists(config).await?;
    }

    let mut options = ConnectOptions::new(config.url());
    options
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(config.connect_timeout)
        .sqlx_logging_level(tracing::log::LevelFilter::Debug);

    let db = Database::connect(options).await?;
    db.ping().await?;

    tracing::info!(
        database = %config.database_name(),
        endpoint = %config.endpoint(),
        "connected to postgres"
    );

    if config.run_migrations {
        Migrator::up(&db, None).await?;
        tracing::info!("migrations are up to date");
    }

    Ok(db)
}

/// Runs `CREATE DATABASE` through the maintenance connection when the target
/// database cannot be reached.
async fn ensure_database_exists(config: &DatabaseConfig) -> Result<(), DbErr> {
    let name = config.database_name();

    if Database::connect(ConnectOptions::new(config.url()))
        .await
        .is_ok()
    {
        return Ok(());
    }

    tracing::warn!(
        database = %name,
        "database unreachable, checking whether it needs to be created"
    );

    let admin = Database::connect(ConnectOptions::new(config.admin_url())).await?;

    let exists = admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT 1 FROM pg_database WHERE datname = $1",
            [Value::from(name.clone())],
        ))
        .await?
        .is_some();

    if exists {
        // The database is there but we could not open it: bad credentials,
        // exhausted connections, ... Let the caller surface the real error.
        return Ok(());
    }

    admin
        .execute_unprepared(&format!("CREATE DATABASE {}", quote_identifier(&name)))
        .await?;

    tracing::info!(database = %name, "database created");

    Ok(())
}

/// Quotes a Postgres identifier so a database name can be interpolated safely.
fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::quote_identifier;

    #[test]
    fn quotes_and_escapes_identifiers() {
        assert_eq!(quote_identifier("rust-axum-askama"), "\"rust-axum-askama\"");
        assert_eq!(quote_identifier("we\"ird"), "\"we\"\"ird\"");
    }
}
