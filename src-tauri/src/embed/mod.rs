pub mod job;

use embed_anything::config::TextEmbedConfig;
use embed_anything::embeddings::embed::{EmbedData, Embedder};
use embed_anything::text_loader::SplittingStrategy;
use rayon::iter::IntoParallelIterator;
use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct AidenTextEmbedder(Arc<Embedder>);

impl AidenTextEmbedder {
    pub fn new(embedder: Embedder) -> Self {
        Self(Arc::new(embedder))
    }

    pub fn config(&self) -> TextEmbedConfig {
        TextEmbedConfig::default()
            .with_chunk_size(256, Some(0.3))
            .with_batch_size(32)
            .with_buffer_size(32)
            .with_splitting_strategy(SplittingStrategy::Sentence)
            .with_semantic_encoder(self.0.clone())
    }
    pub fn embedding<P: AsRef<Path>>(&self, path: P) -> Vec<EmbedData> {
        let mut files = Vec::new();
        let path = path.as_ref();
        if path.is_dir() {
            files.extend(get_files_in_dir(path));
        } else {
            files.push(PathBuf::from(path));
        }

        let mut data = Vec::new();
        files
            .into_par_iter()
            .map(|file| {
                let e = self.clone();
                tauri::async_runtime::block_on(async move { e.embedding_file(file).await })
            })
            .collect_into_vec(&mut data);
        data.into_iter().flatten().flatten().collect::<Vec<_>>()
    }

    pub async fn embedding_file<P: AsRef<Path>>(&self, path: P) -> Option<Vec<EmbedData>> {
        embed_anything::embed_file(path, &self.0, Some(&self.config()), None::<fn(Vec<EmbedData>)>)
            .await
            .ok()
            .flatten()
    }
}

impl Deref for AidenTextEmbedder {
    type Target = Arc<Embedder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn get_files_in_dir<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // 递归获取子目录中的文件
                files.extend(get_files_in_dir(path));
            } else if path.is_file() {
                // 如果是文件，添加到结果中
                files.push(path);
            }
        }
    }
    files
}

fn walk_dir<P: AsRef<Path>>(path: P, callback: &dyn Fn(&Path)) {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir(path, callback);
            } else if path.is_file() {
                callback(&path);
            }
        }
    }
}
