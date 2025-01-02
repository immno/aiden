pub mod errors;
pub mod models;
pub mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle()
                    .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![rag_query, rag_scan_files]) // 注册命令
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::Manager;

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
