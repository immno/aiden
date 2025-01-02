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
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle()
                    .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![rag_query, rag_scan_files, get_sync_list, add_sync_items, delete_sync_item]) // 注册命令
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Manager, State};

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
