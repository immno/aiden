use crate::errors::AppResult;
use crate::storage::DB;
use arrow_array::types::Float32Type;
use arrow_array::{Array, FixedSizeListArray, Int64Array, RecordBatch, RecordBatchIterator, StringArray};
use arrow_schema::{DataType, Field, Schema};
use chrono::Local;
use embed_anything::embeddings::embed::{EmbedData, EmbeddingResult};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::Table;
use std::ops::Deref;
use std::sync::{Arc, LazyLock};

static DEFINE_FILE_CONTENT_SCHEMA: LazyLock<Arc<Schema>> = LazyLock::new(|| {
    Arc::new(Schema::new(vec![
        Field::new("file_path", DataType::Utf8, false),
        Field::new("text", DataType::Utf8, false),
        // 模型不同768或384就不同，根据模型而异
        Field::new(
            "embedding",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, false)), 384),
            false,
        ),
        Field::new("add_time", DataType::Int64, false),
    ]))
});

#[derive(Clone)]
pub struct FileContentsRepo(Table);

impl FileContentsRepo {
    pub async fn new(db: &DB) -> AppResult<Self> {
        let table = db.get_or_crate_table("file_contents", DEFINE_FILE_CONTENT_SCHEMA.clone()).await?;
        Ok(Self(table))
    }

    /// 插入数据
    pub async fn insert_data(&self, records: FileContentRecordFields) -> AppResult<()> {
        let batches = RecordBatch::try_new(
            DEFINE_FILE_CONTENT_SCHEMA.clone(),
            vec![
                Arc::new(StringArray::from(records.file_paths)),
                Arc::new(StringArray::from(records.texts)),
                Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                    records.embeddings.into_iter().map(|v| Some(v.into_iter().map(Some))),
                    384,
                )),
                Arc::new(Int64Array::from(records.add_times)),
            ],
        );

        self.add(RecordBatchIterator::new(vec![batches], DEFINE_FILE_CONTENT_SCHEMA.clone()))
            .execute()
            .await?;
        Ok(())
    }

    pub async fn find_similar(&self, vector: Vec<f32>, n: usize) -> AppResult<FileContentRecords> {
        let results = self.query().nearest_to(vector)?.limit(n).execute().await?.try_collect::<Vec<_>>().await?;

        let records = results.into_iter().flat_map(|row| FileContentRecords::from(row).0).collect();

        Ok(FileContentRecords(records))
    }

    /// 删除数据
    pub async fn delete_by(&self, path: &str) -> AppResult<()> {
        self.delete(&format!("file_path = '{}'", path)).await?;
        Ok(())
    }
}

impl Deref for FileContentsRepo {
    type Target = Table;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct FileContentRecords(pub Vec<FileContentRecord>);

impl Deref for FileContentRecords {
    type Target = Vec<FileContentRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<RecordBatch> for FileContentRecords {
    fn from(batch: RecordBatch) -> Self {
        let mut records = Vec::new();

        // 获取每一列的数据
        let file_path_array = batch.column(0).as_any().downcast_ref::<StringArray>().unwrap();
        let text_array = batch.column(1).as_any().downcast_ref::<StringArray>().unwrap();
        let add_time_array = batch.column(3).as_any().downcast_ref::<Int64Array>().unwrap();

        // 遍历每一行
        for i in 0..batch.num_rows() {
            let file_path = file_path_array.value(i).to_string();
            let text = text_array.value(i).to_string();
            let add_time = add_time_array.value(i);

            records.push(FileContentRecord {
                file_path,
                text,
                embedding: vec![],
                add_time,
            });
        }

        FileContentRecords(records)
    }
}

#[derive(Debug)]
pub struct FileContentRecord {
    pub file_path: String,
    pub text: String,
    pub embedding: Vec<f32>,
    pub add_time: i64,
}

#[derive(Debug, Default)]
pub struct FileContentRecordFields {
    file_paths: Vec<String>,
    texts: Vec<String>,
    embeddings: Vec<Vec<f32>>,
    add_times: Vec<i64>,
}

impl FileContentRecordFields {
    pub fn new(path: String, data: Vec<EmbedData>) -> Self {
        let mut texts = Vec::with_capacity(data.len());
        let mut embeddings = Vec::with_capacity(data.len());
        data.into_iter().filter(|f| f.text.is_some()).for_each(|embed| {
            let emb = match embed.embedding {
                EmbeddingResult::DenseVector(d) => d,
                EmbeddingResult::MultiVector(mut m) => m.is_empty().then_some(vec![]).unwrap_or(m.remove(0)),
            };
            texts.push(embed.text.unwrap_or_default());
            embeddings.push(emb);
        });

        let file_paths = vec![path; texts.len()];
        let add_times = vec![Local::now().timestamp(); texts.len()];
        Self {
            file_paths,
            texts,
            embeddings,
            add_times,
        }
    }
}
