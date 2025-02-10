mod docx;
mod lopdf;

use crate::embed::text_loader::TextLoader;
use crate::errors::{AidenErrors, AppResult};
use crate::extract::docx::DocxRsProcessor;
use crate::extract::lopdf::LoPdfProcessor;
use embed_anything::config::TextEmbedConfig;
use embed_anything::embeddings::embed::{EmbedData, EmbedImage, Embedder, TextEmbedder, VisionEmbedder};
use embed_anything::embeddings::get_text_metadata;
use embed_anything::file_processor::markdown_processor::MarkdownProcessor;
use embed_anything::file_processor::txt_processor::TxtProcessor;
use embed_anything::text_loader::SplittingStrategy;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use std::sync::Arc;
use text_cleaner::clean::Clean;

pub async fn embed_file<T: AsRef<std::path::Path>, F>(
    file_name: T,
    embedder: &Arc<Embedder>,
    config: Option<&TextEmbedConfig>,
    adapter: Option<F>,
) -> anyhow::Result<Option<Vec<EmbedData>>>
where
    F: Fn(Vec<EmbedData>), // Add Send trait bound here
{
    let binding = TextEmbedConfig::default();
    let config = config.unwrap_or(&binding);
    let chunk_size = config.chunk_size.unwrap_or(256);
    let overlap_ratio = config.overlap_ratio.unwrap_or(0.2);
    let batch_size = config.batch_size;
    let splitting_strategy = config.splitting_strategy.unwrap_or(SplittingStrategy::Sentence);
    let semantic_encoder = config.semantic_encoder.clone().unwrap_or(embedder.clone());

    match embedder.as_ref() {
        Embedder::Text(embedder) => {
            emb_text(
                file_name,
                embedder,
                chunk_size,
                overlap_ratio,
                batch_size,
                splitting_strategy,
                semantic_encoder,
                adapter,
            )
            .await
        }
        Embedder::Vision(embedder) => Ok(Some(vec![emb_image(file_name, embedder).await?])),
    }
}

#[allow(clippy::too_many_arguments)]
async fn emb_text<T: AsRef<std::path::Path>, F>(
    file: T,
    embedding_model: &TextEmbedder,
    chunk_size: usize,
    overlap_ratio: f32,
    batch_size: Option<usize>,
    splitting_strategy: SplittingStrategy,
    semantic_encoder: Arc<Embedder>,
    adapter: Option<F>,
) -> anyhow::Result<Option<Vec<EmbedData>>>
where
    F: Fn(Vec<EmbedData>),
{
    let text = extract_text(&file)
        .await?
        .remove_leading_spaces()
        .remove_trailing_spaces()
        .remove_empty_lines();
    let textloader = TextLoader::new(chunk_size, overlap_ratio);

    let chunks = textloader
        .split_into_chunks(&text, splitting_strategy, semantic_encoder)
        .unwrap_or_default();

    let metadata = TextLoader::get_metadata(file).ok();

    if let Some(adapter) = adapter {
        let encodings = embedding_model.embed(&chunks, batch_size).await?;
        let embeddings = get_text_metadata(&Rc::new(encodings), &chunks, &metadata)?;
        adapter(embeddings);
        Ok(None)
    } else {
        let encodings = embedding_model.embed(&chunks, batch_size).await?;
        let embeddings = get_text_metadata(&Rc::new(encodings), &chunks, &metadata)?;

        Ok(Some(embeddings))
    }
}

async fn emb_image<T: AsRef<std::path::Path>>(image_path: T, embedding_model: &VisionEmbedder) -> anyhow::Result<EmbedData> {
    let mut metadata = HashMap::new();
    metadata.insert("file_name".to_string(), fs::canonicalize(&image_path)?.to_str().unwrap().to_string());
    let embedding = embedding_model.embed_image(&image_path, Some(metadata))?;

    Ok(embedding.clone())
}

pub async fn extract_text<T: AsRef<std::path::Path>>(file: &T) -> AppResult<String> {
    if !file.as_ref().exists() {
        return Err(AidenErrors::Str("文件找不到"));
    }
    let file_extension = file.as_ref().extension().unwrap();
    match file_extension.to_str().unwrap() {
        "pdf" => Ok(LoPdfProcessor::extract_text(file).await?),
        "md" => Ok(MarkdownProcessor::extract_text(file)?),
        "txt" => Ok(TxtProcessor::extract_text(file)?),
        "docx" => Ok(DocxRsProcessor::extract_text(file).await?),
        _ => Err(AidenErrors::Str("其他文件格式未实现")),
    }
}
