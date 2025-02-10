use crate::embed::statistical::StatisticalChunker;
use anyhow::Error;
use chrono::{DateTime, Local};
use embed_anything::embeddings::embed::Embedder;
use embed_anything::embeddings::select_device;
use embed_anything::file_processor::docx_processor::DocxProcessor;
use embed_anything::file_processor::markdown_processor::MarkdownProcessor;
use embed_anything::file_processor::pdf_processor::PdfProcessor;
use embed_anything::file_processor::txt_processor::TxtProcessor;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, LazyLock};
use text_splitter::{ChunkConfig, TextSplitter};
use tokenizers::Tokenizer;

const TOKENIZER_JSON: &[u8] = include_bytes!("../../assets/tokenizers/chinese-roberta-wwm-ext-tokenizer.json");
pub static TOKENIZER: LazyLock<Tokenizer> = LazyLock::new(|| Tokenizer::from_bytes(TOKENIZER_JSON).unwrap());

#[derive(Debug)]
pub struct TextLoader {
    pub splitter: TextSplitter<Tokenizer>,
}
impl TextLoader {
    pub fn new(chunk_size: usize, overlap_ratio: f32) -> Self {
        Self {
            splitter: TextSplitter::new(
                ChunkConfig::new(chunk_size)
                    .with_overlap(chunk_size * overlap_ratio as usize)
                    .unwrap()
                    .with_sizer(TOKENIZER.clone()),
            ),
        }
    }
    pub fn split_into_chunks(
        &self,
        text: &str,
        splitting_strategy: embed_anything::text_loader::SplittingStrategy,
        embedder: Arc<Embedder>,
    ) -> Option<Vec<String>> {
        if text.is_empty() {
            return None;
        }

        // Remove single newlines but keep double newlines
        let cleaned_text = text
            .replace("\n\n", "{{DOUBLE_NEWLINE}}")
            .replace("\n", " ")
            .replace("{{DOUBLE_NEWLINE}}", "\n\n");
        let chunks: Vec<String> = match splitting_strategy {
            embed_anything::text_loader::SplittingStrategy::Sentence => {
                self.splitter.chunks(&cleaned_text).par_bridge().map(|chunk| chunk.to_string()).collect()
            }
            embed_anything::text_loader::SplittingStrategy::Semantic => {
                let chunker = StatisticalChunker {
                    encoder: embedder,
                    device: select_device(),
                    threshold_adjustment: 0.01,
                    dynamic_threshold: true,
                    window_size: 5,
                    min_split_tokens: 100,
                    max_split_tokens: 512,
                    split_token_tolerance: 10,
                    tokenizer: TOKENIZER.clone(),
                    verbose: false,
                };

                tokio::task::block_in_place(|| {
                    tokio::runtime::Runtime::new()
                        .unwrap()
                        .block_on(async { chunker.chunk(&cleaned_text, 64).await })
                })
            }
        };

        Some(chunks)
    }

    pub fn extract_text<T: AsRef<std::path::Path>>(file: &T, use_ocr: bool) -> Result<String, Error> {
        if !file.as_ref().exists() {
            return Err(embed_anything::text_loader::FileLoadingError::FileNotFound(file.as_ref().to_str().unwrap().to_string()).into());
        }
        let file_extension = file.as_ref().extension().unwrap();
        match file_extension.to_str().unwrap() {
            "pdf" => PdfProcessor::extract_text(file, use_ocr),
            "md" => MarkdownProcessor::extract_text(file),
            "txt" => TxtProcessor::extract_text(file),
            "docx" => DocxProcessor::extract_text(file),
            _ => Err(embed_anything::text_loader::FileLoadingError::UnsupportedFileType(
                file.as_ref().extension().unwrap().to_str().unwrap().to_string(),
            )
            .into()),
        }
    }

    pub fn get_metadata<T: AsRef<std::path::Path>>(file: T) -> Result<HashMap<String, String>, Error> {
        let metadata = fs::metadata(&file).unwrap();
        let mut metadata_map = HashMap::new();
        metadata_map.insert("created".to_string(), format!("{}", DateTime::<Local>::from(metadata.created()?)));
        metadata_map.insert("modified".to_string(), format!("{}", DateTime::<Local>::from(metadata.modified()?)));

        metadata_map.insert("file_name".to_string(), fs::canonicalize(file)?.to_str().unwrap().to_string());
        Ok(metadata_map)
    }
}
