//! sea-orm-migration MigratorTrait for the chat persistence layer.
//!
//! Each migration file is a `MigrationTrait` impl named after the
//! creation date so they sort lexicographically.

use sea_orm_migration::prelude::*;

mod m2026_05_09_000001_create_chat_envelope;

/// Public Migrator entry-point. Run with
/// `Migrator::up(db, None).await` from a binary, or use the canned
/// `PgChatStore::run_migrations` helper.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(
            m2026_05_09_000001_create_chat_envelope::Migration,
        )]
    }
}
