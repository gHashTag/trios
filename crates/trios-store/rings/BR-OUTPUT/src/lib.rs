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
}
