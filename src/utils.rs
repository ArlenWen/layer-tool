use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use tar::{Archive, Builder};
use walkdir::WalkDir;

/// Compress data using gzip
pub fn compress_data(input: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(input)
        .context("Failed to write data to gzip encoder")?;
    encoder.finish()
        .context("Failed to finish gzip compression")
}

/// Decompress gzip data
pub fn decompress_data(input: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(input);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)
        .context("Failed to decompress gzip data")?;
    Ok(output)
}

/// Compress a file using gzip
pub fn compress_file<P: AsRef<Path>>(input_path: P, output_path: P) -> Result<()> {
    let input_file = File::open(&input_path)
        .with_context(|| format!("Failed to open input file: {:?}", input_path.as_ref()))?;
    let output_file = File::create(&output_path)
        .with_context(|| format!("Failed to create output file: {:?}", output_path.as_ref()))?;

    let mut reader = BufReader::new(input_file);
    let writer = BufWriter::new(output_file);
    let mut encoder = GzEncoder::new(writer, Compression::default());

    std::io::copy(&mut reader, &mut encoder)
        .context("Failed to compress file")?;
    encoder.finish()
        .context("Failed to finish file compression")?;

    Ok(())
}

/// Decompress a gzip file
pub fn decompress_file<P: AsRef<Path>>(input_path: P, output_path: P) -> Result<()> {
    let input_file = File::open(&input_path)
        .with_context(|| format!("Failed to open compressed file: {:?}", input_path.as_ref()))?;
    let output_file = File::create(&output_path)
        .with_context(|| format!("Failed to create output file: {:?}", output_path.as_ref()))?;

    let reader = BufReader::new(input_file);
    let mut writer = BufWriter::new(output_file);
    let mut decoder = GzDecoder::new(reader);

    std::io::copy(&mut decoder, &mut writer)
        .context("Failed to decompress file")?;

    Ok(())
}

/// Create a tar archive from a directory
pub fn create_tar_archive<P: AsRef<Path>>(source_dir: P, output_path: P) -> Result<String> {
    let output_file = File::create(&output_path)
        .with_context(|| format!("Failed to create tar file: {:?}", output_path.as_ref()))?;
    let mut builder = Builder::new(output_file);

    let source_path = source_dir.as_ref();
    if !source_path.exists() {
        return Err(anyhow::anyhow!("Source directory does not exist: {:?}", source_path));
    }

    // Collect and sort entries for consistent checksums
    let mut entries: Vec<_> = WalkDir::new(source_path)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to walk directory")?;

    // Sort entries for consistent checksums (same as calculate_directory_checksum)
    entries.sort_by(|a, b| a.path().cmp(b.path()));

    // Calculate checksum while creating archive
    let mut hasher = Sha256::new();

    for entry in entries {
        let path = entry.path();

        if path.is_file() {
            let relative_path = path.strip_prefix(source_path)
                .context("Failed to create relative path")?;

            // Add file to archive
            builder.append_path_with_name(path, relative_path)
                .with_context(|| format!("Failed to add file to archive: {:?}", path))?;

            // Update checksum (same method as calculate_directory_checksum)
            hasher.update(relative_path.to_string_lossy().as_bytes());

            let mut file = File::open(path)
                .with_context(|| format!("Failed to open file for checksum: {:?}", path))?;
            let mut buffer = [0; 8192];

            loop {
                let bytes_read = file.read(&mut buffer)
                    .with_context(|| format!("Failed to read file: {:?}", path))?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
        } else if path.is_dir() && path != source_path {
            let relative_path = path.strip_prefix(source_path)
                .context("Failed to create relative path")?;

            // Add directory to archive
            builder.append_dir(relative_path, path)
                .with_context(|| format!("Failed to add directory to archive: {:?}", path))?;

            // Update checksum (same method as calculate_directory_checksum)
            hasher.update(relative_path.to_string_lossy().as_bytes());
        }
    }

    builder.finish()
        .context("Failed to finish tar archive")?;

    let checksum = format!("{:x}", hasher.finalize());
    Ok(checksum)
}

/// Extract a tar archive to a directory
pub fn extract_tar_archive<P: AsRef<Path>>(archive_path: P, output_dir: P) -> Result<()> {
    let archive_file = File::open(&archive_path)
        .with_context(|| format!("Failed to open tar file: {:?}", archive_path.as_ref()))?;
    let mut archive = Archive::new(archive_file);

    archive.unpack(&output_dir)
        .with_context(|| format!("Failed to extract tar archive to: {:?}", output_dir.as_ref()))?;

    Ok(())
}

/// Calculate SHA256 checksum of a file
pub fn calculate_file_checksum<P: AsRef<Path>>(file_path: P) -> Result<String> {
    let mut file = File::open(&file_path)
        .with_context(|| format!("Failed to open file for checksum: {:?}", file_path.as_ref()))?;
    
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer)
            .context("Failed to read file for checksum")?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}

/// Calculate SHA256 checksum of a directory (recursive)
pub fn calculate_directory_checksum<P: AsRef<Path>>(dir_path: P) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut entries: Vec<_> = WalkDir::new(&dir_path)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to walk directory")?;
    
    // Sort entries for consistent checksums
    entries.sort_by(|a, b| a.path().cmp(b.path()));
    
    for entry in entries {
        let path = entry.path();
        
        if path.is_file() {
            // Include file path and content in checksum
            let relative_path = path.strip_prefix(&dir_path)
                .context("Failed to create relative path")?;
            hasher.update(relative_path.to_string_lossy().as_bytes());
            
            let mut file = File::open(path)
                .with_context(|| format!("Failed to open file: {:?}", path))?;
            let mut buffer = [0; 8192];
            
            loop {
                let bytes_read = file.read(&mut buffer)
                    .with_context(|| format!("Failed to read file: {:?}", path))?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
        } else if path.is_dir() && path != dir_path.as_ref() {
            // Include directory path in checksum
            let relative_path = path.strip_prefix(&dir_path)
                .context("Failed to create relative path")?;
            hasher.update(relative_path.to_string_lossy().as_bytes());
        }
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}

/// Check if a file is gzip compressed
pub fn is_gzip_file<P: AsRef<Path>>(file_path: P) -> Result<bool> {
    let mut file = File::open(&file_path)
        .with_context(|| format!("Failed to open file: {:?}", file_path.as_ref()))?;
    
    let mut magic = [0u8; 2];
    match file.read_exact(&mut magic) {
        Ok(_) => Ok(magic == [0x1f, 0x8b]),
        Err(_) => Ok(false), // File too short or read error
    }
}

/// Validate file path to prevent directory traversal attacks
pub fn validate_file_path<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    
    // Check for directory traversal attempts
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(anyhow::anyhow!("Path contains parent directory reference: {:?}", path));
            }
            std::path::Component::RootDir => {
                return Err(anyhow::anyhow!("Absolute paths are not allowed: {:?}", path));
            }
            _ => {}
        }
    }
    
    Ok(())
}

/// Create directory if it doesn't exist
pub fn ensure_directory_exists<P: AsRef<Path>>(dir_path: P) -> Result<()> {
    let path = dir_path.as_ref();
    if !path.exists() {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {:?}", path))?;
    }
    Ok(())
}

/// Get file size in bytes
pub fn get_file_size<P: AsRef<Path>>(file_path: P) -> Result<u64> {
    let metadata = std::fs::metadata(&file_path)
        .with_context(|| format!("Failed to get file metadata: {:?}", file_path.as_ref()))?;
    Ok(metadata.len())
}

/// Format file size in human readable format
pub fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}
