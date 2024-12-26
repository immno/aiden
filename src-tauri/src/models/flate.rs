use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use md5::{Context, Digest};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// 拆分并压缩文件
pub fn split_and_compress_file<P: AsRef<Path>>(input_path: P, output_dir: P, chunk_size: usize) -> std::io::Result<Digest> {
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
        hasher.write(&buffer[..bytes_read])?;
        encoder.write_all(&buffer[..bytes_read])?;
        encoder.finish()?;

        part_number += 1;
    }

    Ok(hasher.compute())
}

pub fn decompress_and_merge_files<P: AsRef<Path>>(input_dir: P, output_path: P) -> std::io::Result<Digest> {
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
        hasher.write(&buffer)?;
        // 写入到输出文件
        buffered_writer.write_all(&buffer)?;
        part_number += 1;
    }

    Ok(hasher.compute())
}
