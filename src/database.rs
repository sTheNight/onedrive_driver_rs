use sea_orm::{Database, DatabaseConnection, DbErr};

const DEFAULT_DATABASE_URL: &str = "sqlite://onedrive_driver.sqlite?mode=rwc";

pub async fn init_database() -> Result<DatabaseConnection, DbErr> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
    let db = Database::connect(&database_url).await?;

    sync_schema(&db).await?;

    Ok(db)
}

async fn sync_schema(db: &DatabaseConnection) -> Result<(), DbErr> {
    db.get_schema_registry("onedrive_driver_rs::entity::*")
        .sync(db)
        .await
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    use super::*;

    #[tokio::test]
    async fn syncs_initial_sqlite_schema() -> Result<(), DbErr> {
        let db = Database::connect("sqlite::memory:").await?;

        sync_schema(&db).await?;

        assert!(table_exists(&db, "admin_user").await?);
        assert!(table_exists(&db, "onedrive_config").await?);

        Ok(())
    }

    async fn table_exists(db: &DatabaseConnection, table_name: &str) -> Result<bool, DbErr> {
        let statement = Statement::from_string(
            DatabaseBackend::Sqlite,
            format!(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = '{table_name}'"
            ),
        );

        Ok(db.query_one_raw(statement).await?.is_some())
    }
}
