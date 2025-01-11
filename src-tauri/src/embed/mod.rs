pub mod job;

use embed_anything::config::TextEmbedConfig;
use embed_anything::embeddings::embed::{EmbedData, Embedder, TextEmbedder};
use embed_anything::text_loader::SplittingStrategy;
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use candle_nn::VarBuilder;
use embed_anything::embeddings::local::bert::BertEmbedder;
use embed_anything::embeddings::local::pooling::Pooling;
use embed_anything::embeddings::select_device;
use embed_anything::models::bert::{BertModel, Config, DTYPE};
use tokenizers::{PaddingParams, Tokenizer, TruncationParams};
use crate::errors::AppResult;

#[derive(Clone)]
pub struct AidenTextEmbedder(Arc<Embedder>);

impl AidenTextEmbedder {

    pub fn new(embedder: Embedder) -> Self {
        Self(Arc::new(embedder))
    }
    pub fn from<P: AsRef<Path>>(source: P, model: P) -> AppResult<Self> {
        let config = fs::read_to_string(source.as_ref().join("config.json"))?;

        let config: Config = serde_json::from_str(&config)?;
        let mut tokenizer = Tokenizer::from_file(source.as_ref().join("tokenizer.json")).unwrap();

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

        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[model.as_ref()], DTYPE, &device) }?;

        let model = BertModel::load(vb, &config)?;

        let embedder = BertEmbedder {
            model,
            pooling: Pooling::Mean,
            tokenizer,
        };

        let aiden_embedder = AidenTextEmbedder::new(Embedder::Text(TextEmbedder::Bert(Box::new(embedder))));
        Ok(aiden_embedder)
    }

    pub fn config(&self) -> TextEmbedConfig {
        TextEmbedConfig::default()
            .with_chunk_size(256, Some(0.3))
            .with_batch_size(32)
            .with_buffer_size(32)
            .with_splitting_strategy(SplittingStrategy::Sentence)
            .with_semantic_encoder(self.0.clone())
    }
    pub async fn embedding<P: AsRef<Path>>(&self, path: P) -> Vec<EmbedData> {
        let mut files = Vec::new();
        let path = path.as_ref();
        if path.is_dir() {
            files.extend(get_files_in_dir(path));
        } else {
            files.push(PathBuf::from(path));
        }

        let mut handles = Vec::with_capacity(files.len());
        for file in files {
            let self_clone = self.clone();
            handles.push(tauri::async_runtime::spawn(async move { self_clone.embedding_file(file).await }));
        }

        let mut data = Vec::with_capacity(handles.len());
        for x in handles {
            if let Some(s) = x.await.ok().flatten() {
                data.extend(s);
            }
        }

        data
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;
    use crate::models::flate::decompress_and_merge_files;

    #[tokio::test]
    async fn test_embedding_file() {
        let dir = tempdir().unwrap();

        let model_target_path = dir.as_ref().join("models").join("model.safetensors");
        let model_source_path = Path::new("assets").join("models").join("all-MiniLM-L6-v2");
        decompress_and_merge_files(model_source_path.as_path(), model_target_path.as_path()).unwrap();
        let aiden_embedder = AidenTextEmbedder::from(model_source_path, model_target_path).expect("Failed to create AidenTextEmbedder");

        // let data = aiden_embedder.embedding_file(r"C:\Users\57481\WPSDrive\238828505\WPS云盘\2 公司\11 数据处理引擎\v3.0\CISDigital®工业互联网平台（V3.0）产品操作手册-工业时序数据存算平台-V1.1.doc").await;
        // let data = aiden_embedder.embedding_file(r"C:\Users\57481\Desktop\CISDigital工业互联网平台（V3.1）产品操作手册-数据运维.docx").await;
        // let data = aiden_embedder.embedding_file(r"C:\Users\57481\Desktop\CISDigital®工业互联网平台（V3.0）产品说明书-工业时序数据存算平台-V1.0.pdf").await;
        let data = aiden_embedder.embedding_file(r"C:\Users\57481\Desktop\小知识分享-1203.pdf").await;
        println!("data: {:?}", data);
    }
}
