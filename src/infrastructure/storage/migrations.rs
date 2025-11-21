use tokio_postgres::Client;
use std::fs;
use std::path::Path;

/// Migration manager for PostgreSQL database
pub struct MigrationManager;

impl MigrationManager {
    /// Run all pending migrations
    pub async fn run_migrations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
        Self::create_migrations_table(client).await?;

        let migrations = Self::get_migration_files()?;
        let applied_migrations = Self::get_applied_migrations(client).await?;

        for migration in migrations {
            if !applied_migrations.contains(&migration.name) {
                println!("Applying migration: {}", migration.name);
                Self::apply_migration(client, &migration).await?;
                Self::record_migration(client, &migration.name).await?;
                println!("✅ Migration {} applied successfully", migration.name);
            }
        }

        Ok(())
    }

    /// Create migrations table if it doesn't exist
    async fn create_migrations_table(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
        client.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                applied_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )",
            &[],
        ).await?;
        Ok(())
    }

    /// Get list of migration files
    fn get_migration_files() -> Result<Vec<MigrationFile>, Box<dyn std::error::Error>> {
        let migrations_dir = Path::new("migrations/postgres");
        let mut migrations = Vec::new();

        if !migrations_dir.exists() {
            return Ok(migrations);
        }

        let entries = fs::read_dir(migrations_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("sql") {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or("Invalid migration file name")?;

                // Parse migration name (e.g., "001_initial_schema.sql" -> "001_initial_schema")
                let name = file_name.trim_end_matches(".sql");

                let content = fs::read_to_string(&path)?;

                migrations.push(MigrationFile {
                    name: name.to_string(),
                    content,
                });
            }
        }

        // Sort migrations by name
        migrations.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(migrations)
    }

    /// Get list of already applied migrations
    async fn get_applied_migrations(client: &Client) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let rows = client.query("SELECT name FROM schema_migrations ORDER BY id", &[]).await?;

        let mut applied = Vec::new();
        for row in rows {
            let name: String = row.get(0);
            applied.push(name);
        }

        Ok(applied)
    }

    /// Apply a single migration
    async fn apply_migration(client: &Client, migration: &MigrationFile) -> Result<(), Box<dyn std::error::Error>> {
        // Split migration content by semicolons and execute each statement
        let statements: Vec<&str> = migration.content
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && !s.starts_with("--"))
            .collect();

        for statement in statements {
            if !statement.trim().is_empty() {
                client.execute(statement, &[]).await?;
            }
        }

        Ok(())
    }

    /// Record that a migration has been applied
    async fn record_migration(client: &Client, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        client.execute(
            "INSERT INTO schema_migrations (name) VALUES ($1)",
            &[&name],
        ).await?;
        Ok(())
    }
}

/// Represents a migration file
struct MigrationFile {
    name: String,
    content: String,
}