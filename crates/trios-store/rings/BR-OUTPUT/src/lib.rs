//! BR-OUTPUT — assembles the trios-store rings.
//!
//! Dependency flow: BR-OUTPUT → ST-02 → ST-01 → ST-00.
//! Single entry point `open_and_migrate` that the rest of the backend uses.

pub use trios_store_st00 as types;
pub use trios_store_st01::Store;
pub use trios_store_st02 as migrations;

use anyhow::Result;

/// Open the SQLite database at `path`, apply the schema (idempotent),
/// and return a ready-to-use [`Store`]. This is what `trios-server` calls.
pub async fn open_and_migrate(path: &str) -> Result<Store> {
    let store = Store::open(path).await?;
    trios_store_st02::migrate(&store).await?;
    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn facade_opens_migrates_memory() {
        let store = Store::open_memory().await.unwrap();
        trios_store_st02::migrate(&store).await.unwrap();
        assert!(store.list_agents().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn facade_creates_file_db() {
        let dir = std::env::temp_dir().join(format!("trios-store-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("smoke.db");
        let path_str = path.to_str().unwrap();

        let store = open_and_migrate(path_str).await.unwrap();
        assert!(store.list_agents().await.unwrap().is_empty());
        assert!(path.exists(), "db file must be created on open");

        // re-open the same file: migration is idempotent
        let store2 = open_and_migrate(path_str).await.unwrap();
        assert!(store2.list_agents().await.unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
