pub mod file_contents;
pub mod files;
pub mod open_ai;

use crate::errors::AppResult;
use arrow_schema::SchemaRef;
use lancedb::{Connection, Table};
use std::sync::Arc;

#[derive(Clone)]
pub struct DB(pub Arc<Connection>);

impl DB {
    pub async fn new(db_path: &str) -> AppResult<DB> {
        Ok(Self(Arc::new(lancedb::connect(db_path).execute().await?)))
    }

    pub async fn get_or_crate_table<S: Into<SchemaRef>>(&self, table_name: &str, schema: S) -> AppResult<Table> {
        let table_exists = self.0.table_names().execute().await?.contains(&table_name.to_string());
        if !table_exists {
            self.0.create_empty_table(table_name, schema.into()).execute().await?;
        }
        let table = self.0.open_table(table_name).execute().await?;
        Ok(table)
    }
}
