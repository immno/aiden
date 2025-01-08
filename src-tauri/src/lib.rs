pub mod embed;
pub mod errors;
pub mod models;
pub mod storage;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            sync_list: Mutex::new(Vec::new()), // 初始化状态
        })
        .setup(init_plugin())
        .setup(init_models())
        .setup(init_lancedb())
        .invoke_handler(tauri::generate_handler![
            rag_query,
            rag_scan_files,
            get_sync_list,
            add_sync_items,
            delete_sync_item
        ]) // 注册命令
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

        app.manage(AidenTextEmbedder::new(Embedder::Text(TextEmbedder::Bert(Box::new(embedder)))));

        Ok(())
    }
}

fn init_lancedb() -> fn(&mut App) -> Result<(), Box<dyn Error>> {
    |app| {
        // 获取资源目录（打包后的 assets 目录）
        let app_path = app.path().app_data_dir().expect("Failed to get app dir");
        let db = tauri::async_runtime::block_on(async move { DB::new(app_path.join("db").to_string_lossy().as_ref()).await });

        // 将模型路径存储到应用状态中
        app.manage(db?);
        Ok(())
    }
}
use tokenizers::TruncationParams;
use tokenizers::PaddingParams;
use tokenizers::Tokenizer;
use crate::errors::AppResult;
use crate::models::flate::decompress_and_merge_files;
use crate::storage::DB;
use candle_nn::VarBuilder;
use embed_anything::embeddings::local::bert::BertEmbedder;
use embed_anything::embeddings::local::pooling::Pooling;
use embed_anything::embeddings::select_device;
use embed_anything::models::bert::{BertModel, Config, DTYPE};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Mutex;
use embed_anything::embeddings::embed::{Embedder, TextEmbedder};
use tauri::{App, Manager, State};
use crate::embed::AidenTextEmbedder;

#[derive(Serialize, Deserialize)]
struct RagResponse {
    success: bool,
    message: String,
}

/// RAG 查询命令
#[tauri::command]
fn rag_query(query: String) -> Result<String, String> {
    // 在这里实现 RAG 查询逻辑
    Ok(format!("Response to: {}", query))
}

/// RAG 文件扫描命令
#[tauri::command]
fn rag_scan_files(file_paths: Vec<String>) -> Result<RagResponse, String> {
    // 在这里实现文件扫描逻辑
    Ok(RagResponse {
        success: true,
        message: format!("Scanned {} files", file_paths.len()),
    })
}

#[derive(Serialize, Deserialize, Clone)]
struct SyncItem {
    name: String,
    add_time: String,
    sync_time: String,
    progress: u32,
}

struct AppState {
    sync_list: Mutex<Vec<SyncItem>>,
}

#[tauri::command]
fn get_sync_list(state: State<AppState>) -> Vec<SyncItem> {
    state.sync_list.lock().unwrap().clone()
}

#[tauri::command]
fn add_sync_items(items: Vec<SyncItem>, state: State<AppState>) {
    let mut sync_list = state.sync_list.lock().unwrap();
    sync_list.extend(items);
}

#[tauri::command]
fn delete_sync_item(index: usize, state: State<AppState>) {
    let mut sync_list = state.sync_list.lock().unwrap();
    if index < sync_list.len() {
        sync_list.remove(index);
    }
}
