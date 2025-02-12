use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use md5::Context;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// 拆分并压缩文件
pub fn split_and_compress_file<P: AsRef<Path>>(input_path: P, output_dir: P, chunk_size: usize) -> std::io::Result<String> {
    // 打开输入文件
    let mut input_file = File::open(input_path)?;
    let mut buffer = vec![0u8; chunk_size];
    let mut hasher = Context::new();
    let mut part_number = 1;

    loop {
        // 从输入文件读取一个分片
        let bytes_read = input_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        // 创建分片文件
        let output_file = File::create(PathBuf::new().join(output_dir.as_ref()).join(format!("part_{}.gz", part_number)))?;
        let buffered_writer = BufWriter::new(output_file);

        // 压缩分片并写入文件
        let mut encoder = GzEncoder::new(buffered_writer, Compression::fast());
        hasher.write_all(&buffer[..bytes_read])?;
        encoder.write_all(&buffer[..bytes_read])?;
        encoder.finish()?;

        part_number += 1;
    }

    Ok(format!("{:x}", hasher.compute()))
}

pub fn decompress_and_merge_files<P: AsRef<Path>>(input_dir: P, output_path: P) -> std::io::Result<String> {
    // 确保目录存在
    if let Some(parent_dir) = output_path.as_ref().parent() {
        std::fs::create_dir_all(parent_dir)?;
    };
    let output_file = File::create(output_path)?;
    let mut buffered_writer = BufWriter::new(output_file);
    let mut hasher = Context::new();
    let mut part_number = 1;

    loop {
        // 打开分片文件
        let path = PathBuf::new().join(input_dir.as_ref()).join(format!("part_{}.gz", part_number));
        if !path.exists() || !path.is_file() {
            break;
        }
        let input_file = File::open(path)?;
        let buffered_reader = BufReader::new(input_file);

        // 解压分片
        let mut decoder = GzDecoder::new(buffered_reader);
        let mut buffer = Vec::new();
        decoder.read_to_end(&mut buffer)?;
        hasher.write_all(&buffer)?;
        // 写入到输出文件
        buffered_writer.write_all(&buffer)?;
        part_number += 1;
    }

    Ok(format!("{:x}", hasher.compute()))
}

pub fn calculate_md5(file_path: &Path) -> std::io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut hasher = Context::new();
    let mut buffer = [0; 1024];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.write_all(&buffer[..bytes_read])?;
    }

    Ok(format!("{:x}", hasher.compute()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_split_and_compress_file() {
        let input_path = "D:\\RustroverProjects\\aiden\\src-tauri\\assets\\models\\albert-chinese-base\\model.safetensors";
        let output_path = "D:\\RustroverProjects\\aiden\\src-tauri\\assets\\models\\albert-chinese-base";

        let ins = Instant::now();
        let md5 = split_and_compress_file(input_path, output_path, 1024 * 1024).unwrap();
        println!("ins: {:?}, MD5: {}", ins.elapsed(), md5);
    }

    #[test]
    fn test_decompress_and_merge_files() {
        let input_dir = "D:\\RustroverProjects\\aiden\\src-tauri\\assets\\models\\all-MiniLM-L6-v2";
        let output_path = "D:\\RustroverProjects\\aiden\\src-tauri\\assets\\models\\all-MiniLM-L6-v2\\model222.safetensors";

        let ins = Instant::now();
        let md5 = decompress_and_merge_files(input_dir, output_path).unwrap();
        println!("ins: {:?}, MD5: {}", ins.elapsed(), md5);
    }
}
