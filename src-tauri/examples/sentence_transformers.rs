// pub mod models;
// pub mod storage;
// pub mod errors;
//
// use clap::Parser;
// use models::bert::BertModelWrapper;
// use rayon::prelude::*;
// use std::fs::{self, File};
// use std::io::{BufRead, BufReader};
// use std::sync::mpsc::{Receiver, Sender};
// use tokio::task::JoinHandle;
// use tracing::{info, warn};
//
// #[derive(Parser)]
// #[command(author, version, about, long_about = None)]
// pub struct Cli {
//     #[clap(short, long)]
//     input_directory: String,
//     #[clap(short, long)]
//     db_uri: String,
// }
//
// struct EmbeddingEntry {
//     filename: String,
//     embedding: Vec<f32>,
// }
//
// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     init_tracing();
//     let cli_args = Cli::parse();
//     let ts_mark = std::time::Instant::now();
//
//     // init the channel that sends data to the thread that writes embedding to the db
//     let (sender, reciever) = std::sync::mpsc::channel::<EmbeddingEntry>();
//     // start the task and get a handle to it
//     let db_writer_task =
//         init_db_writer_task(reciever, cli_args.db_uri.as_str(), "vectors_table_1", 100).await?;
//
//     // list the files in the directory to be embedded
//     let files_dir = fs::read_dir(cli_args.input_directory)?;
//
//     let file_list = files_dir
//         .into_iter()
//         .map(|file| file.unwrap().path().to_str().unwrap().to_string())
//         .collect::<Vec<String>>();
//     // process the files in parallel
//     file_list.par_iter().for_each(|filename| {
//         if let Err(e) = process_text_file(sender.clone(), filename.as_str()) {
//             warn!("Error processing file: {}: Error:{}", filename, e)
//         }
//     });
//
//     drop(sender); // this will close the original channel
//     info!("All files processed, waiting for write task to finish");
//     db_writer_task.await?; // wait for the db writer task to finish before exiting
//     info!(
//         "{} files indexed in: {:?}",
//         file_list.len(),
//         ts_mark.elapsed()
//     );
//     Ok(())
// }
//
// // process a text file and send the embeddings to the channel
// fn process_text_file(sender: Sender<EmbeddingEntry>, filename: &str) -> anyhow::Result<()> {
//     let bert_model = models::bert::get_model_reference()?;
//     info!("reading file: {}", filename);
//     let text_chunks = read_file_in_chunks(filename, 256)?;
//     let text_chunks: Vec<&str> = text_chunks.iter().map(AsRef::as_ref).collect();
//     let file_vector = embed_multiple_sentences(&text_chunks, false, &bert_model)?;
//     sender.send(EmbeddingEntry {
//         filename: filename.to_string(),
//         embedding: file_vector[0].clone(),
//     })?;
//     Ok(())
// }
//
// /// Initialize the task that writes the embeddings to the db
// /// ## Arguments
// /// * reciever: the channel that receives the embeddings
// /// * db_uri: the uri of the db e.g. data/vecdb
// /// * table_name: the name of the table to write the embeddings to
// async fn init_db_writer_task(
//     reciever: Receiver<EmbeddingEntry>,
//     db_uri: &str,
//     table_name: &str,
//     buffer_size: usize,
// ) -> anyhow::Result<JoinHandle<()>> {
//     let db = storage::VecDB::connect(db_uri, table_name).await?;
//     let task_handle = tokio::spawn(async move {
//         let mut embeddings_buffer = Vec::new();
//         while let Ok(embedding) = reciever.recv() {
//             embeddings_buffer.push(embedding);
//             if embeddings_buffer.len() >= buffer_size {
//                 let (keys, vectors) = extract_keys_and_vectors(&embeddings_buffer);
//                 db.add_vector(&keys, vectors, 384).await.unwrap();
//                 embeddings_buffer.clear();
//             }
//         }
//         if !embeddings_buffer.is_empty() {
//             let (keys, vectors) = extract_keys_and_vectors(&embeddings_buffer);
//             db.add_vector(&keys, vectors, 384).await.unwrap();
//         }
//     });
//     Ok(task_handle)
// }
//
// fn extract_keys_and_vectors(embeddings_buffer: &[EmbeddingEntry]) -> (Vec<&str>, Vec<Vec<f32>>) {
//     embeddings_buffer
//         .iter()
//         .map(|entry| (entry.filename.as_str(), entry.embedding.clone()))
//         .unzip::<&str, Vec<f32>, Vec<&str>, Vec<Vec<f32>>>()
// }
//
// fn embed_multiple_sentences(
//     sentences: &[&str],
//     apply_mean: bool,
//     bert_model: &BertModelWrapper,
// ) -> anyhow::Result<Vec<Vec<f32>>> {
//     let multiple_embeddings = bert_model.embed_sentences(sentences, apply_mean)?;
//     if apply_mean {
//         let multiple_embeddings = multiple_embeddings.to_vec1::<f32>()?;
//         Ok(vec![multiple_embeddings])
//     } else {
//         let multiple_embeddings = multiple_embeddings.to_vec2::<f32>()?;
//         Ok(multiple_embeddings)
//     }
// }
//
// fn embed_sentence(sentence: &str, bert_model: &BertModelWrapper) -> anyhow::Result<Vec<f32>> {
//     let embedding = bert_model.embed_sentence(sentence)?;
//     println!("embedding Tensor: {:?}", embedding);
//     // we squeeze the tensor to remove the batch dimension
//     let embedding = embedding.squeeze(0)?;
//     println!("embedding Tensor after squeeze: {:?}", embedding);
//     let embedding = embedding.to_vec1::<f32>().unwrap();
//     //println!("embedding Vec: {:?}", embedding);
//     Ok(embedding)
// }
//
// fn init_tracing() {
//     if let Ok(level_filter) = tracing_subscriber::EnvFilter::try_from_env("LOG_LEVEL") {
//         tracing_subscriber::fmt()
//             .with_env_filter(level_filter)
//             .with_ansi(true)
//             .with_file(true)
//             .with_line_number(true)
//             .init();
//     } else {
//         println!("Failed to parse LOG_LEVEL env variable, using default log level: INFO");
//         tracing_subscriber::fmt()
//             .with_ansi(true)
//             .with_file(true)
//             .with_line_number(true)
//             .init();
//     }
// }
//
// fn read_file_in_chunks(file_path: &str, chunk_size: usize) -> anyhow::Result<Vec<String>> {
//     let file = File::open(file_path).unwrap();
//     let reader = BufReader::new(file);
//     let mut sentences = Vec::new();
//     let mut text_buffer = String::new();
//     for text in reader.lines() {
//         let text = text?;
//         text_buffer.push_str(text.as_str());
//         let word_count = text_buffer.split_whitespace().count();
//         if word_count >= chunk_size {
//             sentences.push(text_buffer.clone());
//             text_buffer.clear();
//         }
//     }
//     if !text_buffer.is_empty() {
//         sentences.push(text_buffer.clone());
//     }
//     Ok(sentences)
// }
//
// // test the entire flow with files in embedding_files_test folder end-to-end
// #[cfg(test)]
// mod tests {
//     use arrow_array::{Array, FixedSizeListArray, Float32Array, StringArray};
//     use super::*;
//     use crate::embed_sentence;
//     #[tokio::test]
//     async fn test_full_flow() {
//         let temp_folder = "temp_test_folder";
//         let temp_table = "temp_test_table";
//         fs::create_dir(temp_folder).unwrap();
//         let (test_sender, test_reciever) = std::sync::mpsc::channel::<EmbeddingEntry>();
//         let db_writer_task = init_db_writer_task(test_reciever, temp_folder, temp_table, 100)
//             .await
//             .unwrap();
//         let files_dir = fs::read_dir("embedding_files_test").unwrap();
//         let file_list = files_dir
//             .into_iter()
//             .map(|file| file.unwrap().path().to_str().unwrap().to_string())
//             .collect::<Vec<String>>();
//         // process the files in parallel
//         file_list.par_iter().for_each(|filename| {
//             if let Err(e) = process_text_file(test_sender.clone(), filename.as_str()) {
//                 panic!("Error processing file: {}: Error:{}", filename, e)
//             }
//         });
//         drop(test_sender); // this will close the original channel
//         db_writer_task.await.unwrap();
//         let db = storage::VecDB::connect(temp_folder, temp_table)
//             .await
//             .unwrap();
//         let bert_model = models::bert::get_model_reference().unwrap();
//         let animals_vector =
//             embed_sentence("生产运维的核心技术创新是什么", &bert_model).unwrap();
//         let record_batch = db.find_similar(animals_vector, 10).await.unwrap();
//
//         let files_array = record_batch.column_by_name("vector").unwrap();
//         let files = files_array.as_any().downcast_ref::<FixedSizeListArray>().unwrap();
//
//         let filenames_array = record_batch.column_by_name("filename").unwrap();
//         let filenames = filenames_array.as_any().downcast_ref::<StringArray>().unwrap();
//
//         for i in 0..files.len() {
//             // 提取 vector 数据
//             let item_list = files.value(i);
//             let float_array = item_list.as_any().downcast_ref::<Float32Array>().unwrap();
//             let vector_items: Vec<f32> = float_array.values().to_vec();
//
//             // 提取 filename 数据
//             let filename = filenames.value(i);
//
//             // 打印结果
//             println!("Filename {}: {}", i, filename);
//             println!("Vector {}: {:?}", i, vector_items);
//         }
//
//         // let files_array = record_batch.column_by_name("filename").unwrap();
//         // let v = files.value(0);
//         // assert_eq!(v, "embedding_files_test/embedding_content_99996.txt");
//         fs::remove_dir_all(temp_folder).unwrap();
//     }
// }

use std::{iter::once, sync::Arc};
use std::str::FromStr;
use std::sync::atomic::AtomicPtr;
use arrow_array::{RecordBatch, RecordBatchIterator, StringArray};
use arrow_schema::{DataType, Field, Schema};
use candle::Device;
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use futures::StreamExt;
use lancedb::{
    arrow::IntoArrow,
    connect,
    embeddings::{
        EmbeddingFunction,
    },
    query::{ExecutableQuery, QueryBase},
    Result,
};
use lancedb::embeddings::EmbeddingDefinition;
use tokenizers::Tokenizer;
use app_lib::errors::AppResult;
use app_lib::models::transformers::SentenceTransformersEmbeddings;

const MODEL: &str = "/home/mno/RustroverProjects/doc-embedder/modes/all-MiniLM-L6-v2/model.safetensors";
const CONFIG_JSON: &str = include_str!("../modes/all-MiniLM-L6-v2/config.json");
const TOKENIZER_JSON: &str = include_str!("../modes/all-MiniLM-L6-v2/tokenizer.json");


#[tokio::main]
async fn main() -> AppResult<()> {
    let tempdir = tempfile::tempdir().unwrap();
    let tempdir = tempdir.path().to_str().unwrap();
    // let embedding = SentenceTransformersEmbeddings::builder()
    //     .model("/home/mno/RustroverProjects/doc-embedder/modes/all-MiniLM-L6-v2/model.safetensors")
    //     .tokenizer_path("/home/mno/RustroverProjects/doc-embedder/modes/all-MiniLM-L6-v2/tokenizer.json")
    //     .config_path("/home/mno/RustroverProjects/doc-embedder/modes/all-MiniLM-L6-v2/config.json")
    //     .build()?;

    let config: Config = serde_json::from_str(CONFIG_JSON)?;
    let tokenizer = Tokenizer::from_str(TOKENIZER_JSON).unwrap();
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[MODEL], DTYPE, &Device::Cpu)? };
    let model = BertModel::load(vb, &config)?;

    let embedding = SentenceTransformersEmbeddings::new(model, tokenizer, Device::Cpu, None);
    let embedding = Arc::new(embedding);
    let db = connect(tempdir).execute().await?;
    db.embedding_registry()
        .register("sentence-transformers", embedding.clone())?;

    let table = db
        .create_table("vectors", make_data())
        .add_embedding(EmbeddingDefinition::new(
            "facts",
            "sentence-transformers",
            Some("embeddings"),
        ))?
        .execute()
        .await?;

    let query = Arc::new(StringArray::from_iter_values(once(
        "How many bones are in the human body?",
    )));
    let query_vector = embedding.compute_query_embeddings(query)?;
    let mut results = table
        .vector_search(query_vector)?
        .limit(3)
        .execute()
        .await?;

    let rb = results.next().await.unwrap()?;
    let out = rb
        .column_by_name("facts")
        .unwrap()
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let text = out.iter().next().unwrap().unwrap();
    println!("Answer: {}", text);
    Ok(())
}

fn make_data() -> impl IntoArrow {
    let schema = Schema::new(vec![Field::new("facts", DataType::Utf8, false)]);

    let facts = StringArray::from_iter_values(vec![
        "Albert Einstein was a theoretical physicist.",
        "The capital of France is Paris.",
        "The Great Wall of China is one of the Seven Wonders of the World.",
        "Python is a popular programming language.",
        "Mount Everest is the highest mountain in the world.",
        "Leonardo da Vinci painted the Mona Lisa.",
        "Shakespeare wrote Hamlet.",
        "The human body has 206 bones.",
        "The speed of light is approximately 299,792 kilometers per second.",
        "Water boils at 100 degrees Celsius.",
        "The Earth orbits the Sun.",
        "The Pyramids of Giza are located in Egypt.",
        "Coffee is one of the most popular beverages in the world.",
        "Tokyo is the capital city of Japan.",
        "Photosynthesis is the process by which plants make their food.",
        "The Pacific Ocean is the largest ocean on Earth.",
        "Mozart was a prolific composer of classical music.",
        "The Internet is a global network of computers.",
        "Basketball is a sport played with a ball and a hoop.",
        "The first computer virus was created in 1983.",
        "Artificial neural networks are inspired by the human brain.",
        "Deep learning is a subset of machine learning.",
        "IBM's Watson won Jeopardy! in 2011.",
        "The first computer programmer was Ada Lovelace.",
        "The first chatbot was ELIZA, created in the 1960s.",
    ]);
    let schema = Arc::new(schema);
    let rb = RecordBatch::try_new(schema.clone(), vec![Arc::new(facts)]).unwrap();
    Box::new(RecordBatchIterator::new(vec![Ok(rb)], schema))
}

