use crate::embed::AidenTextEmbedder;
use crate::storage::file_contents::{FileContentRecordFields, FileContentsRepo};
use crate::storage::files::FilesRepo;
use embed_anything::embeddings::embed::EmbedData;
use flume::{Receiver, Sender};
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
pub struct EmbedManager {
    tx: Sender<(String, Vec<EmbedData>)>,
    rx: Option<Receiver<(String, Vec<EmbedData>)>>,
}

impl EmbedManager {
    pub fn start_embedding(&mut self, repo: FilesRepo, embedder: AidenTextEmbedder) {
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                let repo2 = repo.clone();
                if let Ok(fr) = repo2.query_progress_zero(10).await {
                    if fr.is_empty() {
                        sleep(Duration::from_millis(3000)).await;
                    } else {
                        for file in fr {
                            let data = embedder.embedding(Path::new(&file.file_path)).await;
                            let _ = tx.send_async((file.file_path, data)).await;
                        }
                    }
                } else {
                    sleep(Duration::from_millis(3000)).await;
                }
            }
        });
    }

    pub fn start_write_embedding(&mut self, files: FilesRepo, repo: FileContentsRepo) {
        let rx = self.rx.take().unwrap();
        tauri::async_runtime::spawn(async move {
            while let Ok((file_path, data)) = rx.recv_async().await {
                if !data.is_empty() {
                    if let Err(e) = repo.insert_data(FileContentRecordFields::new(file_path.clone(), data)).await {
                        log::error!("Failed to insert data, {}: {}", file_path, e);
                    }
                    if let Err(e) = files.update_progress_and_sync_time(&file_path, 100).await {
                        log::error!("Failed to update progress, {}: {}", file_path, e);
                    }
                }
            }
        });
    }
}

impl Default for EmbedManager {
    fn default() -> Self {
        let (tx, rx) = flume::bounded(10000);

        Self { tx, rx: Some(rx) }
    }
}
