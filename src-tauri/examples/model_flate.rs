use std::time::Instant;
use app_lib::models::flate::{decompress_and_merge_files, split_and_compress_file};

fn main() -> std::io::Result<()> {
    let input_path = "/home/mno/RustroverProjects/aiden/src-tauri/modes/all-MiniLM-L6-v2/model.safetensors";
    let output_path = "/home/mno/RustroverProjects/aiden/src-tauri/modes/all-MiniLM-L6-v2/";

    let ins = Instant::now();
    let md5 = split_and_compress_file(input_path, output_path, 1024 * 1024)?;
    println!("ins: {:?}, MD5: {:?}", ins.elapsed(), md5);

    let input_dir = "/home/mno/RustroverProjects/aiden/src-tauri/modes/all-MiniLM-L6-v2/";
    let output_path = "/home/mno/RustroverProjects/aiden/src-tauri/modes/all-MiniLM-L6-v2/model222.safetensors";
    let md5 = decompress_and_merge_files(input_dir, output_path)?;
    println!("ins: {:?}, MD5: {:?}", ins.elapsed(), md5);

    Ok(())
}
