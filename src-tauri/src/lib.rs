pub mod embed;
pub mod errors;
pub mod models;
pub mod storage;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(init_plugin())
        .setup(init_lancedb())
        .setup(init_models())
        .invoke_handler(tauri::generate_handler![rag_query, get_sync_list, add_sync_items, delete_sync_item]) // 注册命令
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_plugin() -> fn(&mut App) -> Result<(), Box<dyn Error>> {
    |app| {
        app.handle().plugin(tauri_plugin_dialog::init())?;
        app.handle()
            .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())?;
        Ok(())
    }
}

fn init_models() -> fn(&mut App) -> Result<(), Box<dyn Error>> {
    |app| {
        // 获取资源目录（打包后的 assets 目录）
        let resource_path = app.path().resource_dir().expect("Failed to get resource dir");
        let model_source_path = resource_path.join("all-MiniLM-L6-v2");
        let app_path = app.path().app_data_dir().expect("Failed to get app dir");
        let model_target_path = app_path.join("models/all-MiniLM-L6-v2").join("model.safetensors");

        decompress_and_merge_files(model_source_path.clone(), model_target_path.clone())?;

        let config = std::fs::read_to_string(model_source_path.clone().join("config.json"))?;
        let config: Config = serde_json::from_str(&config)?;
        let mut tokenizer = Tokenizer::from_file(model_source_path.clone().join("tokenizer.json")).unwrap();

        let pp = PaddingParams {
            strategy: tokenizers::PaddingStrategy::BatchLongest,
            ..Default::default()
        };
        let trunc = TruncationParams {
            strategy: tokenizers::TruncationStrategy::LongestFirst,
            max_length: config.max_position_embeddings,
            ..Default::default()
        };

        tokenizer.with_padding(Some(pp)).with_truncation(Some(trunc)).unwrap();

        let device = select_device();

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[model_target_path], DTYPE, &device) }?;

        let model = BertModel::load(vb, &config)?;

        let embedder = BertEmbedder {
            model,
            pooling: Pooling::Mean,
            tokenizer,
        };

        let aiden_embedder = AidenTextEmbedder::new(Embedder::Text(TextEmbedder::Bert(Box::new(embedder))));

        app.manage(aiden_embedder.clone());

        let files = app.state::<FilesRepo>().inner().clone();
        let file_contexts = app.state::<FileContentsRepo>().inner().clone();

        let mut manager = EmbedManager::default();
        manager.start_embedding(files.clone(), aiden_embedder);
        manager.start_write_embedding(files, file_contexts);

        Ok(())
    }
}

fn init_lancedb() -> fn(&mut App) -> Result<(), Box<dyn Error>> {
    |app| {
        // 获取资源目录（打包后的 assets 目录）
        let app_path = app.path().app_data_dir().expect("Failed to get app dir");
        let db = tauri::async_runtime::block_on(async move { DB::new(app_path.join("db").to_string_lossy().as_ref()).await })?;

        let db2 = db.clone();
        let files_db = tauri::async_runtime::block_on(async move { FilesRepo::new(&db2).await })?;

        let db3 = db.clone();
        let file_context_db = tauri::async_runtime::block_on(async move { FileContentsRepo::new(&db3).await })?;

        app.manage(file_context_db);
        app.manage(files_db);
        app.manage(db);
        Ok(())
    }
}

use crate::embed::job::EmbedManager;
use crate::embed::AidenTextEmbedder;
use crate::errors::AppResult;
use crate::models::flate::decompress_and_merge_files;
use crate::storage::file_contents::FileContentsRepo;
use crate::storage::files::{FileRecord, FilesRepo};
use crate::storage::DB;
use candle_nn::VarBuilder;
use embed_anything::embeddings::embed::{Embedder, EmbeddingResult, TextEmbedder};
use embed_anything::embeddings::local::bert::BertEmbedder;
use embed_anything::embeddings::local::pooling::Pooling;
use embed_anything::embeddings::select_device;
use embed_anything::models::bert::{BertModel, Config, DTYPE};
use std::collections::HashMap;
use std::error::Error;
use tauri::{App, Manager, State};
use tokenizers::PaddingParams;
use tokenizers::Tokenizer;
use tokenizers::TruncationParams;

/// RAG 查询命令
#[tauri::command]
fn rag_query(query: String, file_context: State<'_, FileContentsRepo>, emb: State<'_, AidenTextEmbedder>) -> AppResult<String> {
    let file_context = file_context.inner().clone();
    let emb = emb.inner().clone();
    tauri::async_runtime::block_on(async move {
        let mut s = emb.embed(&[query], emb.config().batch_size).await?;
        if s.is_empty() {
            Ok("".to_string())
        } else {
            let v = match s.remove(0) {
                EmbeddingResult::DenseVector(d) => d,
                EmbeddingResult::MultiVector(mut m) => m.remove(0),
            };
            let records = file_context.find_similar(v, 5).await?;
            let mut map: HashMap<String, Vec<String>> = HashMap::new();

            for record in records.0 {
                map.entry(record.file_path).or_insert_with(Vec::new).push(record.text);
            }

            let rep = map
                .into_iter()
                .map(|(k, v)| format!("文件：{}\n 内容：{}", k, v.join("\n")))
                .collect::<Vec<_>>()
                .join("\n");
            Ok(rep)
        }
    })
}

#[tauri::command]
fn get_sync_list(state: State<'_, FilesRepo>) -> AppResult<Vec<FileRecord>> {
    let state = state.inner().clone();
    tauri::async_runtime::block_on(async move { state.query_all().await })
}

#[tauri::command]
fn add_sync_items(items: Vec<String>, state: State<'_, FilesRepo>) -> AppResult<()> {
    let state = state.inner().clone();
    tauri::async_runtime::block_on(async move { state.insert_data(items).await })
}

#[tauri::command]
fn delete_sync_item(path: String, state: State<'_, FilesRepo>) -> AppResult<()> {
    let state = state.inner().clone();
    tauri::async_runtime::block_on(async move { state.delete_by(&path).await })
}
