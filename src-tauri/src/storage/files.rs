use crate::errors::AppResult;
use crate::storage::DB;
use arrow_array::{Array, Int64Array, RecordBatch, RecordBatchIterator, StringArray, UInt32Array};
use arrow_schema::{DataType, Field, Schema};
use chrono::Local;
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::Table;
use std::ops::Deref;
use std::sync::{Arc, LazyLock};

static DEFINE_TABLE_SCHEMA: LazyLock<Arc<Schema>> = LazyLock::new(|| {
    Arc::new(Schema::new(vec![
        Field::new("name".to_string(), DataType::Utf8, false),
        Field::new("file_path".to_string(), DataType::Utf8, false),
        Field::new("file_type".to_string(), DataType::Utf8, true),
        Field::new("add_time".to_string(), DataType::Int64, false),
        Field::new("sync_time".to_string(), DataType::Int64, false),
        Field::new("progress".to_string(), DataType::UInt32, false),
    ]))
});

pub struct FilesRepo(Table);

impl FilesRepo {
    pub async fn new(db: &DB) -> AppResult<Self> {
        let table = db.get_or_crate_table("files", DEFINE_TABLE_SCHEMA.clone()).await?;
        Ok(Self(table))
    }

    /// 插入数据
    pub async fn insert_data(&self, paths: Vec<String>) -> AppResult<()> {
        let mut records = FileRecordFields::default();

        for path in paths {
            let path_obj = std::path::Path::new(&path);
            let name = path_obj.file_name().unwrap_or_else(|| path_obj.as_os_str()).to_string_lossy().to_string();

            let file_type = if path_obj.is_file() {
                path_obj.extension().map(|ext| ext.to_string_lossy().to_string())
            } else {
                None
            };

            records.names.push(name);
            records.file_paths.push(path);
            records.file_types.push(file_type);
            records.add_times.push(Local::now().timestamp());
            records.sync_times.push(0);
            records.progresses.push(0);
        }

        let batches = RecordBatch::try_new(
            DEFINE_TABLE_SCHEMA.clone(),
            vec![
                Arc::new(StringArray::from(records.names)),
                Arc::new(StringArray::from(records.file_paths)),
                Arc::new(StringArray::from(records.file_types)),
                Arc::new(Int64Array::from(records.add_times)),
                Arc::new(Int64Array::from(records.sync_times)),
                Arc::new(UInt32Array::from(records.progresses)),
            ],
        );

        self.add(RecordBatchIterator::new(vec![batches], DEFINE_TABLE_SCHEMA.clone()))
            .execute()
            .await?;
        Ok(())
    }

    /// 查询全部数据
    pub async fn query_all(&self) -> AppResult<Vec<FileRecord>> {
        let results = self.query().execute().await?.try_collect::<Vec<_>>().await?;
        let records = results.into_iter().map(|row| FileRecords::from(row).0).flatten().collect();

        Ok(records)
    }

    /// 查询 progress = 0 的数据
    pub async fn query_progress_zero(&self) -> AppResult<Vec<FileRecord>> {
        let results = self.query().only_if("progress = 0").execute().await?.try_collect::<Vec<_>>().await?;

        let records = results.into_iter().map(|row| FileRecords::from(row).0).flatten().collect();

        Ok(records)
    }

    pub async fn update_progress_and_sync_time(&self, file_path: &str, new_progress: u32) -> AppResult<()> {
        let new_sync_time = Local::now().timestamp();

        self.update()
            .only_if(format!("file_path = '{}'", file_path))
            .column("progress", new_progress.to_string())
            .column("sync_time", new_sync_time.to_string())
            .execute()
            .await?;

        Ok(())
    }
}

impl Deref for FilesRepo {
    type Target = Table;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct FileRecords(Vec<FileRecord>);

impl Deref for FileRecords {
    type Target = Vec<FileRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct FileRecord {
    pub name: String,
    pub file_path: String,
    /// 文件类型可能为空（目录没有文件类型）
    pub file_type: Option<String>,
    pub add_time: i64,
    pub sync_time: i64,
    pub progress: u32,
}

impl From<RecordBatch> for FileRecords {
    fn from(batch: RecordBatch) -> Self {
        let mut records = Vec::new();

        // 获取每一列的数据
        let name_array = batch.column(0).as_any().downcast_ref::<StringArray>().unwrap();
        let file_path_array = batch.column(1).as_any().downcast_ref::<StringArray>().unwrap();
        let file_type_array = batch.column(2).as_any().downcast_ref::<StringArray>().unwrap();
        let add_time_array = batch.column(3).as_any().downcast_ref::<Int64Array>().unwrap();
        let sync_time_array = batch.column(4).as_any().downcast_ref::<Int64Array>().unwrap();
        let progress_array = batch.column(5).as_any().downcast_ref::<UInt32Array>().unwrap();

        // 遍历每一行
        for i in 0..batch.num_rows() {
            let name = name_array.value(i).to_string();
            let file_path = file_path_array.value(i).to_string();
            let file_type = if file_type_array.is_null(i) {
                None
            } else {
                Some(file_type_array.value(i).to_string())
            };
            let add_time = add_time_array.value(i);
            let sync_time = sync_time_array.value(i);
            let progress = progress_array.value(i);

            records.push(FileRecord {
                name,
                file_path,
                file_type,
                add_time,
                sync_time,
                progress,
            });
        }

        FileRecords(records)
    }
}

#[derive(Debug, Default)]
struct FileRecordFields {
    names: Vec<String>,
    file_paths: Vec<String>,
    file_types: Vec<Option<String>>,
    add_times: Vec<i64>,
    sync_times: Vec<i64>,
    progresses: Vec<u32>,
}

#[cfg(test)]
mod lancedb_files_tests {
    use super::*;
    use std::path::Path;
    use tempfile::{tempdir, TempDir};

    async fn repo(dir: &TempDir) -> FilesRepo {
        let db_path = dir.path().join("test_db");
        let db = DB::new(db_path.to_str().unwrap()).await.unwrap();
        FilesRepo::new(&db).await.unwrap()
    }

    fn file_paths() -> Vec<String> {
        vec![
            Path::new("test_files").to_string_lossy().into_owned(),
            Path::new("test_files").join("test.md").to_string_lossy().into_owned(),
        ]
    }

    // 测试 insert_data 方法
    #[tokio::test]
    async fn test_insert_data() {
        let dir = tempdir().unwrap();
        let repo = repo(&dir).await;

        let paths = file_paths();
        let result = repo.insert_data(paths).await;
        assert!(result.is_ok(), "Failed to insert data");
    }

    // 测试 query_all 方法
    #[tokio::test]
    async fn test_query_all() {
        let dir = tempdir().unwrap();
        let repo = repo(&dir).await;

        // 插入数据
        let paths = file_paths();
        repo.insert_data(paths).await.unwrap();

        // 查询全部数据
        let records = repo.query_all().await.unwrap();
        assert_eq!(records.len(), 2);

        // 验证查询结果
        assert_eq!(&records[0].name, "test_files");
        assert_eq!(&records[0].file_path, &Path::new("test_files").to_string_lossy().to_string());
        assert_eq!(&records[0].file_type, &None);
        assert!(&records[0].add_time > &0);
        assert_eq!(&records[0].sync_time, &0);
        assert_eq!(&records[0].progress, &0);

        assert_eq!(&records[1].name, "test.md");
        assert_eq!(
            &records[1].file_path,
            &Path::new("test_files").join("test.md").to_string_lossy().to_string()
        );
        assert_eq!(&records[1].file_type, &Some("md".to_string()));
        assert!(&records[0].add_time > &0);
        assert_eq!(&records[0].sync_time, &0);
        assert_eq!(&records[0].progress, &0);
    }

    // 测试 query_progress_zero 方法
    #[tokio::test]
    async fn test_query_progress_zero() {
        let dir = tempdir().unwrap();
        let repo = repo(&dir).await;

        // 插入数据
        let paths = file_paths();
        repo.insert_data(paths.clone()).await.unwrap();

        // 查询 progress = 0 的数据
        let records = repo.query_progress_zero().await.unwrap();
        assert_eq!(records.len(), 2);

        // 验证查询结果
        assert_eq!(&records[0].name, "test_files");
        assert_eq!(&records[0].file_path, &Path::new("test_files").to_string_lossy().to_string());
        assert_eq!(&records[0].file_type, &None);
        assert!(&records[0].add_time > &0);
        assert_eq!(&records[0].sync_time, &0);
        assert_eq!(&records[0].progress, &0);

        assert_eq!(&records[1].name, "test.md");
        assert_eq!(
            &records[1].file_path,
            &Path::new("test_files").join("test.md").to_string_lossy().to_string()
        );
        assert_eq!(&records[1].file_type, &Some("md".to_string()));
        assert!(&records[0].add_time > &0);
        assert_eq!(&records[0].sync_time, &0);
        assert_eq!(&records[0].progress, &0);
    }

    // 测试 update_progress_and_sync_time 方法
    #[tokio::test]
    async fn test_update_progress_and_sync_time() {
        let dir = tempdir().unwrap();
        let repo = repo(&dir).await;

        // 插入数据
        let paths = file_paths();
        repo.insert_data(paths).await.unwrap();

        // 更新 progress 和 sync_time
        let new_progress = 100;
        let s = Path::new("test_files").join("test.md").to_string_lossy().to_string();
        let result = repo.update_progress_and_sync_time(&s, new_progress).await;

        assert!(result.is_ok(), "Failed to update progress and sync_time");

        // 查询全部数据以验证更新
        let records = repo.query_all().await.unwrap();
        assert_eq!(records.len(), 2);

        // 验证查询结果
        assert_eq!(&records[0].name, "test_files");
        assert_eq!(&records[0].file_path, &Path::new("test_files").to_string_lossy().to_string());
        assert_eq!(&records[0].file_type, &None);
        assert!(&records[0].add_time > &0);
        assert_eq!(&records[0].sync_time, &0);
        assert_eq!(&records[0].progress, &0);

        assert_eq!(&records[1].name, "test.md");
        assert_eq!(
            &records[1].file_path,
            &Path::new("test_files").join("test.md").to_string_lossy().to_string()
        );
        assert_eq!(&records[1].file_type, &Some("md".to_string()));
        assert!(&records[1].add_time > &0);
        assert!(&records[1].sync_time> &0);
        assert_eq!(&records[1].progress, &100);
    }
}
