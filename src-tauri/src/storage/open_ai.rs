use crate::errors::AppResult;
use crate::storage::DB;
use arrow_array::{Array, BooleanArray, Int64Array, RecordBatch, RecordBatchIterator, StringArray};
use arrow_schema::{DataType, Field, Schema};
use chrono::Local;
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::Table;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::{Arc, LazyLock};

static DEFINE_OPEN_AI_SCHEMA: LazyLock<Arc<Schema>> = LazyLock::new(|| {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("url", DataType::Utf8, false),
        Field::new("token", DataType::Utf8, false),
        Field::new("state", DataType::Boolean, false),
        Field::new("time", DataType::Int64, false),
    ]))
});

#[derive(Clone)]
pub struct OpenAiRepo(Table);

impl OpenAiRepo {
    pub async fn new(db: &DB) -> AppResult<Self> {
        let table = db.get_or_crate_table("open_ai", DEFINE_OPEN_AI_SCHEMA.clone()).await?;
        Ok(Self(table))
    }

    /// 插入数据
    pub async fn insert_data(&self, url: String, token: String) -> AppResult<()> {
        let batches = RecordBatch::try_new(
            DEFINE_OPEN_AI_SCHEMA.clone(),
            vec![
                Arc::new(Int64Array::from(vec![1])),
                Arc::new(StringArray::from(vec![url])),
                Arc::new(StringArray::from(vec![token])),
                Arc::new(BooleanArray::from(vec![true])),
                Arc::new(Int64Array::from(vec![Local::now().timestamp()])),
            ],
        );

        self.add(RecordBatchIterator::new(vec![batches], DEFINE_OPEN_AI_SCHEMA.clone()))
            .execute()
            .await?;
        Ok(())
    }

    /// 查询全部数据
    pub async fn query_all(&self) -> AppResult<Vec<OpenAiRecord>> {
        let results = self.query().execute().await?.try_collect::<Vec<_>>().await?;
        let records = results.into_iter().flat_map(|row| OpenAiRecords::from(row).0).collect();

        Ok(records)
    }

    /// 查询全部数据
    pub async fn query_id(&self) -> AppResult<Option<OpenAiRecord>> {
        let results = self.query().only_if("id = 1").limit(1).execute().await?.try_collect::<Vec<_>>().await?;
        let records = results.into_iter().flat_map(|row| OpenAiRecords::from(row).0).collect::<Vec<_>>();

        Ok(records.into_iter().nth(0))
    }

    /// 查询可用状态的数据
    pub async fn query_available(&self, limit: usize) -> AppResult<Vec<OpenAiRecord>> {
        let results = self
            .query()
            .only_if("state = true")
            .limit(limit)
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;

        let records = results.into_iter().flat_map(|row| OpenAiRecords::from(row).0).collect();

        Ok(records)
    }

    /// 更新 token
    pub async fn update_insert_token(&self, url: &str, new_token: &str) -> AppResult<()> {
        if self.query_id().await?.is_none() {
            self.insert_data(url.to_string(), new_token.to_string()).await?;
        } else {
            self.update_token(url, new_token).await?;
        }
        Ok(())
    }
    /// 更新 token
    pub async fn update_token(&self, url: &str, new_token: &str) -> AppResult<()> {
        self.update()
            .only_if("id = 1")
            .column("url", format!("'{}'", url))
            .column("state", "true")
            .column("token", format!("'{}'", new_token))
            .column("time", Local::now().timestamp().to_string())
            .execute()
            .await?;

        Ok(())
    }

    /// 更新状态为不可用
    pub async fn update_state(&self, state: bool) -> AppResult<()> {
        self.update()
            .only_if("id = 1")
            .column("state", state.to_string())
            .column("time", Local::now().timestamp().to_string())
            .execute()
            .await?;

        Ok(())
    }
}

impl Deref for OpenAiRepo {
    type Target = Table;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct OpenAiRecords(Vec<OpenAiRecord>);

impl Deref for OpenAiRecords {
    type Target = Vec<OpenAiRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenAiRecord {
    pub id: i64,
    pub url: String,
    pub token: String,
    pub state: bool,
    pub time: i64,
}

impl From<RecordBatch> for OpenAiRecords {
    fn from(batch: RecordBatch) -> Self {
        let mut records = Vec::new();

        // 获取每一列的数据
        let id_array = batch.column(0).as_any().downcast_ref::<Int64Array>().unwrap();
        let url_array = batch.column(1).as_any().downcast_ref::<StringArray>().unwrap();
        let token_array = batch.column(2).as_any().downcast_ref::<StringArray>().unwrap();
        let state_array = batch.column(3).as_any().downcast_ref::<BooleanArray>().unwrap();
        let time_array = batch.column(4).as_any().downcast_ref::<Int64Array>().unwrap();

        // 遍历每一行
        for i in 0..batch.num_rows() {
            let id = id_array.value(i);
            let url = url_array.value(i).to_string();
            let token = token_array.value(i).to_string();
            let state = state_array.value(i);
            let time = time_array.value(i);

            records.push(OpenAiRecord { id, url, token, state, time });
        }

        OpenAiRecords(records)
    }
}

#[cfg(test)]
mod lancedb_openai_tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    async fn repo(dir: &TempDir) -> OpenAiRepo {
        let db_path = dir.path().join("test_db");
        let db = DB::new(db_path.to_str().unwrap()).await.unwrap();
        OpenAiRepo::new(&db).await.unwrap()
    }

    fn test_data() -> (String, String) {
        ("https://api.deepseek.com/v1".to_string(), "token1".to_string())
    }

    // 测试 insert_data 方法
    #[tokio::test]
    async fn test_insert_data() {
        let dir = tempdir().unwrap();
        let repo = repo(&dir).await;

        let (urls, tokens) = test_data();
        let result = repo.insert_data(urls, tokens).await;
        assert!(result.is_ok(), "Failed to insert data");
    }

    // 测试 query_all 方法
    #[tokio::test]
    async fn test_query_all() {
        let dir = tempdir().unwrap();
        let repo = repo(&dir).await;

        // 插入数据
        let (urls, tokens) = test_data();
        repo.insert_data(urls, tokens).await.unwrap();

        // 查询全部数据
        let records = repo.query_all().await.unwrap();
        assert_eq!(records.len(), 1);

        // 验证查询结果
        assert_eq!(&records[0].url, "https://api.deepseek.com/v1");
        assert_eq!(&records[0].token, "token1");
        assert_eq!(&records[0].state, &true);
        assert!(&records[0].time > &0);
    }

    // 测试 query_available 方法
    #[tokio::test]
    async fn test_query_available() {
        let dir = tempdir().unwrap();
        let repo = repo(&dir).await;

        // 插入数据
        let (urls, tokens) = test_data();
        repo.insert_data(urls, tokens).await.unwrap();

        // 查询可用状态的数据
        let records = repo.query_available(10).await.unwrap();
        assert_eq!(records.len(), 1);

        // 验证查询结果
        assert_eq!(&records[0].url, "https://api.deepseek.com/v1");
        assert_eq!(&records[0].token, "token1");
        assert_eq!(&records[0].state, &true);
        assert!(&records[0].time > &0);
    }

    // 测试 update_token 方法
    #[tokio::test]
    async fn test_update_token() {
        let dir = tempdir().unwrap();
        let repo = repo(&dir).await;

        // 插入数据
        let (urls, tokens) = test_data();
        repo.insert_data(urls, tokens).await.unwrap();

        // 更新 token
        let _ = repo.update_token("https://api.deepseek.com/v2", "new_token2").await.unwrap();

        // 查询全部数据以验证更新
        let records = repo.query_all().await.unwrap();
        assert_eq!(records.len(), 1);

        // 验证查询结果
        assert_eq!(&records[0].url, "https://api.deepseek.com/v2");
        assert_eq!(&records[0].token, "new_token2");
        assert_eq!(&records[0].state, &true);
        assert!(&records[0].time > &0);
    }

    // 测试 update_state_unavailable 方法
    #[tokio::test]
    async fn test_update_state_unavailable() {
        let dir = tempdir().unwrap();
        let repo = repo(&dir).await;

        // 插入数据
        let (urls, tokens) = test_data();
        repo.insert_data(urls, tokens).await.unwrap();

        // 更新状态为不可用
        let result = repo.update_state(false).await;
        assert!(result.is_ok(), "Failed to update state");

        // 查询全部数据以验证更新
        let records = repo.query_all().await.unwrap();
        assert_eq!(records.len(), 1);

        // 验证查询结果
        assert_eq!(&records[0].url, "https://api.deepseek.com/v1");
        assert_eq!(&records[0].token, "token1");
        assert_eq!(&records[0].state, &false);
        assert!(&records[0].time > &0);
    }
}
