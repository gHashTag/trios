//! Neon PostgreSQL connection and database operations

use sqlx::PgPool;
use anyhow::Result;
use tracing::info;

/// Neon database connection
pub struct NeonDb {
    pool: PgPool,
}

impl NeonDb {
    /// Create new Neon connection from connection string
    pub async fn connect(conn_str: &str) -> Result<Self> {
        info!("Connecting to Neon database...");
        
        let pool = PgPool::connect(conn_str).await?;
        
        info!("Connected to Neon database successfully");
        
        Ok(Self { pool })
    }
    
    /// Get connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    
    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");
        
        let migration_sql = std::include_str!("../migrations/001_initial.sql");
        sqlx::query(migration_sql).execute(&self.pool).await?;
        
        info!("Migrations completed successfully");
        
        Ok(())
    }
    
    /// Initialize database (run migrations if needed)
    pub async fn initialize(&self) -> Result<()> {
        // Check if tables exist
        let table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'igla_race_trials')"
        )
        .fetch_one(&self.pool)
        .await?;
        
        if !table_exists {
            self.run_migrations().await?;
        } else {
            info!("Database already initialized");
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_neon_db_creation() {
        // Unit test only - requires real connection string for integration tests
    }
}
