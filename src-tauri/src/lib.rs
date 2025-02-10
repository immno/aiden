pub mod agent;
pub mod embed;
pub mod errors;
pub mod extract;
pub mod models;
pub mod storage;

use crate::agent::OpenAiAgent;
use crate::embed::job::EmbedManager;
use crate::embed::AidenTextEmbedder;
use crate::errors::AppResult;
use crate::models::flate::{calculate_md5, decompress_and_merge_files};
use crate::storage::file_contents::FileContentsRepo;
use crate::storage::files::{FileRecord, FilesRepo};
use crate::storage::open_ai::OpenAiRepo;
use crate::storage::DB;
use embed_anything::embeddings::embed::EmbeddingResult;
use lancedb::table::OptimizeAction;
use log::info;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;
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
        .invoke_handler(tauri::generate_handler![
            rag_query,
            get_sync_list,
            add_sync_items,
            delete_sync_item,
            get_ai_config,
            save_ai_config
        ]) // 注册命令
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
                let _ = files.optimize(OptimizeAction::All).await;
                let _ = file_contexts.optimize(OptimizeAction::All).await;
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

    let db4 = db.clone();
    let open_ai_db = tauri::async_runtime::block_on(async move { OpenAiRepo::new(&db4).await })?;

    app.manage(file_context_db);
    app.manage(files_db);
    app.manage(open_ai_db);
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
async fn rag_query(
    query: String,
    ai: State<'_, OpenAiRepo>,
    file_context: State<'_, FileContentsRepo>,
    emb: State<'_, AidenTextEmbedder>,
) -> AppResult<String> {
    let question = query.clone();
    let mut s = emb.embed(&[query], emb.config().batch_size).await?;
    if s.is_empty() {
        Ok("请输入内容或问题".to_string())
    } else {
        let v = match s.remove(0) {
            EmbeddingResult::DenseVector(d) => d,
            EmbeddingResult::MultiVector(mut m) => m.remove(0),
        };
        let records = file_context.find_similar(v, 5).await?;
        let res = if let Some(rt) = ai.query_id().await? {
            if rt.state {
                let agent = OpenAiAgent::new(rt.url.as_ref(), rt.token.as_ref());
                match agent.query(question.as_str(), &records).await {
                    Ok(r) => r,
                    Err(_) => {
                        let _ = ai.update_state(false).await;
                        records.to_markdown()
                    }
                }
            } else {
                records.to_markdown()
            }
        } else {
            records.to_markdown()
        };
        Ok(res)
    }
}

#[tauri::command]
async fn get_sync_list(state: State<'_, FilesRepo>) -> AppResult<Vec<FileRecord>> {
    let state = state.inner().clone();
    state.query_all().await
}

#[tauri::command]
async fn add_sync_items(items: Vec<String>, state: State<'_, FilesRepo>) -> AppResult<()> {
    state.insert_data(items).await
}

#[tauri::command]
async fn delete_sync_item(path: String, files: State<'_, FilesRepo>, contents: State<'_, FileContentsRepo>) -> AppResult<()> {
    let _ = files.delete_by(&path).await;
    contents.delete_by(&path).await
}

#[tauri::command]
async fn get_ai_config(ai: State<'_, OpenAiRepo>) -> AppResult<OpenAiConfig> {
    let res = ai.query_id().await?;
    Ok(res.map(|r| OpenAiConfig { url: r.url, token: r.token }).unwrap_or_default())
}

#[tauri::command]
async fn save_ai_config(config: OpenAiConfig, ai: State<'_, OpenAiRepo>) -> AppResult<()> {
    let _ = ai.update_insert_token(config.url.as_ref(), config.token.as_ref()).await;
    Ok(())
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct OpenAiConfig {
    pub url: String,
    pub token: String,
}
