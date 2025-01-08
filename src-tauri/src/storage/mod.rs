mod files;

use crate::errors::AppResult;
use arrow_schema::SchemaRef;
use lancedb::{Connection, Table};
use std::ops::Deref;
use std::sync::Arc;

pub struct DB(Arc<Connection>);

impl DB {
    pub async fn new(db_path: &str) -> AppResult<DB> {
        Ok(Self(Arc::new(lancedb::connect(db_path).execute().await?)))
    }

    pub async fn get_or_crate_table<S: Into<SchemaRef>>(&self, table_name: &str, schema: S) -> AppResult<Table> {
        let table_exists = self.table_names().execute().await?.contains(&table_name.to_string());
        if !table_exists {
            self.create_empty_table(table_name, schema.into()).execute().await?;
        }
        let table = self.open_table(table_name).execute().await?;
        Ok(table)
    }
}

impl Deref for DB {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}