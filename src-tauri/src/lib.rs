pub mod embed;
pub mod errors;
pub mod models;
pub mod storage;

use crate::embed::job::EmbedManager;
use crate::embed::AidenTextEmbedder;
use crate::errors::AppResult;
use crate::models::flate::{calculate_md5, decompress_and_merge_files};
use crate::storage::file_contents::FileContentsRepo;
use crate::storage::files::{FileRecord, FilesRepo};
use crate::storage::DB;
use embed_anything::embeddings::embed::EmbeddingResult;
use log::info;
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use lancedb::table::OptimizeAction;
use tauri::{App, Manager, State};
use tauri_plugin_log::{Target, TargetKind};
use tokio::time::sleep;

const MODEL_MD5: &str = "5d228912c417f6abf7732710314ddeed";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .target(Target::new(TargetKind::LogDir { file_name: None }))
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(init_setup())
        .invoke_handler(tauri::generate_handler![rag_query, get_sync_list, add_sync_items, delete_sync_item]) // 注册命令
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_setup() -> fn(&mut App) -> Result<(), Box<dyn Error>> {
    |app| {
        init_lancedb(app)?;
        init_models(app)?;

        let aiden_embedder = app.state::<AidenTextEmbedder>().inner().clone();
        let files = app.state::<FilesRepo>().inner().clone();
        let file_contexts = app.state::<FileContentsRepo>().inner().clone();

        let mut manager = EmbedManager::default();
        manager.start_embedding(files.clone(), aiden_embedder);
        manager.start_write_embedding(files.clone(), file_contexts.clone());

        tauri::async_runtime::spawn(async move {
            loop {
                let _  = files.optimize(OptimizeAction::All).await;
                let _  = file_contexts.optimize(OptimizeAction::All).await;
                sleep(Duration::from_secs(3600)).await;
            }
        });

        Ok(())
    }
}

fn init_lancedb(app: &mut App) -> Result<(), Box<dyn Error>> {
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

fn init_models(app: &mut App) -> Result<(), Box<dyn Error>> {
    let resource_path = app.path().resource_dir().expect("Failed to get resource dir");
    let model_source_path = resource_path.join("assets").join("models").join("all-MiniLM-L6-v2");
    let app_data_path = app.path().app_data_dir().expect("Failed to get app data dir");
    let model_target_path = app_data_path.join("models").join("model.safetensors");

    let md5 = if model_target_path.exists() {
        let mut md5 = calculate_md5(model_target_path.as_path()).unwrap_or("".to_string());
        if !md5.eq(MODEL_MD5) {
            let _ = std::fs::remove_file(model_target_path.as_path());
            md5 = decompress_and_merge_files(model_source_path.as_path(), model_target_path.as_path())?;
        }
        md5
    } else {
        decompress_and_merge_files(model_source_path.as_path(), model_target_path.as_path())?
    };

    if !md5.eq(MODEL_MD5) {
        info!("Model Md5 Err: {:?}", model_target_path);
    }

    let aiden_embedder = AidenTextEmbedder::from(model_source_path, model_target_path).expect("Failed to create AidenTextEmbedder");
    app.manage(aiden_embedder);

    Ok(())
}

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
            if records.0.is_empty() {
                return Ok("对不起，无数据!".to_string());
            }
            let mut map: HashMap<String, Vec<String>> = HashMap::new();

            for record in records.0 {
                map.entry(record.file_path).or_default().push(record.text);
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
fn delete_sync_item(path: String, files: State<'_, FilesRepo>, contents: State<'_, FileContentsRepo>) -> AppResult<()> {
    let path_c = path.clone();
    let files = files.inner().clone();
    let contents = contents.inner().clone();
    let _ = tauri::async_runtime::block_on(async move { files.delete_by(&path).await });
    tauri::async_runtime::block_on(async move { contents.delete_by(&path_c).await })
}
