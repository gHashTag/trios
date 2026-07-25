//! SeaORM entities mirroring the drizzle SQLite schema (ST-02 DDL) 1:1.
//!
//! Enum-typed columns (`adapter`, `detected_by`) stay `String` at this
//! layer; the repository maps them to/from the ST-00 enums so the wire
//! format in the database is identical to what the TS backend wrote.

pub mod agent_definitions {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "agent_definitions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub adapter: String,
        pub model_id: String,
        pub reasoning_effort: String,
        pub permission_mode: String,
        pub session_key: String,
        pub pinned: bool,
        pub adapter_config_json: Option<String>,
        pub created_at: i64,
        pub updated_at: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod oauth_tokens {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "oauth_tokens")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub browseros_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub provider: String,
        pub access_token: String,
        pub refresh_token: String,
        pub expires_at: i64,
        pub email: Option<String>,
        pub account_id: Option<String>,
        pub updated_at: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod produced_files {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "produced_files")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub agent_definition_id: String,
        pub session_key: String,
        pub turn_id: String,
        pub turn_prompt: String,
        pub path: String,
        pub size: i64,
        pub mtime_ms: i64,
        pub created_at: i64,
        pub detected_by: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
