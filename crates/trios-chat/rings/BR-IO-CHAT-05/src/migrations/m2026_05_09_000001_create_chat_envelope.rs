//! Migration: create `chat_envelope` (Wave-3 trinity-fpga#33).
//!
//! Creates the table and the `dest_hash` index used for sealed-sender
//! routing.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(ChatEnvelope::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(ChatEnvelope::SessionId)
                        .binary()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ChatEnvelope::Counter)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ChatEnvelope::DestHash)
                        .binary()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ChatEnvelope::Ciphertext)
                        .binary()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ChatEnvelope::StoredAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .primary_key(
                    Index::create()
                        .col(ChatEnvelope::SessionId)
                        .col(ChatEnvelope::Counter),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("chat_envelope_dest_hash_idx")
                .table(ChatEnvelope::Table)
                .col(ChatEnvelope::DestHash)
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_index(
            Index::drop()
                .name("chat_envelope_dest_hash_idx")
                .table(ChatEnvelope::Table)
                .to_owned(),
        )
        .await?;
        m.drop_table(Table::drop().table(ChatEnvelope::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum ChatEnvelope {
    Table,
    SessionId,
    Counter,
    DestHash,
    Ciphertext,
    StoredAt,
}
