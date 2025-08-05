use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;
use tar::Archive;
use tempfile::TempDir;

use crate::docker::DockerClient;
use crate::output::*;
use crate::types::ExportData;
use crate::utils::{
    decompress_file, extract_tar_archive, is_gzip_file,
    calculate_directory_checksum, format_file_size, get_file_size
};

pub struct ImportCommand {
    docker_client: DockerClient,
}

impl ImportCommand {
    pub fn new() -> Self {
        Self {
            docker_client: DockerClient::new(),
        }
    }

    /// Import layer data from export file to container
    pub fn execute(&self, input_path: &str, container_id: &str, backup: bool) -> Result<()> {
        print_progress(&format!("Starting import to container: {}", container_id));

        let input_file_path = Path::new(input_path);
        if !input_file_path.exists() {
            return Err(anyhow::anyhow!("Input file not found: {}", input_path));
        }

        // Validate target container exists and is ready for layer operations
        print_progress("Validating target container state...");
        self.docker_client.validate_container_for_layer_operations(container_id)
            .context("Target container validation failed")?;

        let file_size = get_file_size(input_file_path)?;
        print_file_info("Input file", input_path, &format_file_size(file_size));

        // Create temporary directory for extraction
        let temp_dir = TempDir::new()
            .context("Failed to create temporary directory")?;
        let temp_path = temp_dir.path();

        // Handle decompression if needed
        let export_tar_path = if is_gzip_file(input_file_path)? {
            print_progress("Decompressing input file...");
            let decompressed_path = temp_path.join("export.tar");
            decompress_file(input_file_path, &decompressed_path)
                .context("Failed to decompress input file")?;
            decompressed_path
        } else {
            input_file_path.to_path_buf()
        };

        // Extract export archive
        print_progress("Extracting export archive...");
        let extract_dir = temp_path.join("extracted");
        std::fs::create_dir_all(&extract_dir)
            .context("Failed to create extraction directory")?;

        self.extract_export_archive(&export_tar_path, &extract_dir)
            .context("Failed to extract export archive")?;

        // Read and validate metadata
        print_progress("Reading export metadata...");
        let metadata_path = extract_dir.join("metadata.json");
        if !metadata_path.exists() {
            return Err(anyhow::anyhow!("Export metadata not found in archive"));
        }

        let metadata_content = std::fs::read_to_string(&metadata_path)
            .context("Failed to read metadata file")?;
        let export_data: ExportData = serde_json::from_str(&metadata_content)
            .context("Failed to parse export metadata")?;

        // Validate layer archive exists
        let layer_tar_path = extract_dir.join("layer.tar");
        if !layer_tar_path.exists() {
            return Err(anyhow::anyhow!("Layer archive not found in export"));
        }

        // Get target container's upper layer path
        print_progress("Locating target container layer directory...");
        let target_upper_path = self.docker_client.get_upper_layer_path(container_id)
            .context("Failed to get target container layer path")?;

        // Backup existing upper layer if it exists and is not empty (when backup is enabled)
        if backup && target_upper_path.exists() {
            let entries = std::fs::read_dir(&target_upper_path)
                .context("Failed to read target upper layer directory")?;

            if entries.count() > 0 {
                let backup_path = target_upper_path.with_extension("backup");
                print_warning(&format!("Backing up existing layer to: {:?}", backup_path));

                if backup_path.exists() {
                    std::fs::remove_dir_all(&backup_path)
                        .context("Failed to remove existing backup")?;
                }

                std::fs::rename(&target_upper_path, &backup_path)
                    .context("Failed to backup existing layer")?;
            }
        } else if !backup && target_upper_path.exists() {
            // Remove existing layer without backup when backup is disabled
            print_warning("Removing existing layer without backup (--no-backup specified)");
            std::fs::remove_dir_all(&target_upper_path)
                .context("Failed to remove existing layer")?;
        }

        // Create target directory
        std::fs::create_dir_all(&target_upper_path)
            .context("Failed to create target upper layer directory")?;

        // Extract layer data to target location
        print_progress("Extracting layer data to container...");
        extract_tar_archive(&layer_tar_path, &target_upper_path)
            .context("Failed to extract layer data to target container")?;

        // Verify checksum
        print_progress("Verifying layer integrity...");
        let calculated_checksum = calculate_directory_checksum(&target_upper_path)
            .context("Failed to calculate imported layer checksum")?;

        if calculated_checksum != export_data.layer_checksum {
            return Err(anyhow::anyhow!(
                "Layer checksum verification failed: expected {}, got {}",
                export_data.layer_checksum,
                calculated_checksum
            ));
        }

        print_success("Import completed successfully!");
        print_container_info("Source container", &export_data.container_metadata.name, &export_data.container_metadata.id);
        print_labeled_value("Target container", container_id);
        print_labeled_value("Image", &export_data.container_metadata.image);
        print_checksum("Layer checksum verified", &calculated_checksum);

        // Display import summary
        self.display_import_summary(&export_data)?;

        Ok(())
    }

    /// Extract the export archive (metadata + layer tar)
    fn extract_export_archive(&self, archive_path: &Path, output_dir: &Path) -> Result<()> {
        let archive_file = File::open(archive_path)
            .context("Failed to open export archive")?;
        let mut archive = Archive::new(archive_file);

        archive.unpack(output_dir)
            .context("Failed to extract export archive")?;

        Ok(())
    }

    /// Display summary of imported data
    fn display_import_summary(&self, export_data: &ExportData) -> Result<()> {
        print_section_header("Import Summary");
        print_labeled_value("Export version", &export_data.version);
        print_labeled_value("Export created", &export_data.created.format("%Y-%m-%d %H:%M:%S UTC").to_string());
        print_info("Source container:");
        print_metadata_item("ID", &export_data.container_metadata.id);
        print_metadata_item("Name", &export_data.container_metadata.name);
        print_metadata_item("Image", &export_data.container_metadata.image);
        print_metadata_item("Image SHA256", &export_data.container_metadata.image_sha256);
        print_metadata_item("Created", &export_data.container_metadata.created.format("%Y-%m-%d %H:%M:%S UTC").to_string());
        print_metadata_item("State", &export_data.container_metadata.state);

        if !export_data.container_metadata.labels.is_empty() {
            print_metadata_item("Labels", "");
            for (key, value) in &export_data.container_metadata.labels {
                print_nested_metadata_item(key, value);
            }
        }

        if !export_data.container_metadata.mounts.is_empty() {
            print_metadata_item("Mounts", &format!("{} mount(s)", export_data.container_metadata.mounts.len()));
        }

        print_info("Docker environment:");
        print_metadata_item("Storage driver", &export_data.docker_info.driver);
        print_metadata_item("Operating system", &export_data.docker_info.operating_system);
        print_metadata_item("Architecture", &export_data.docker_info.architecture);
        print_metadata_item("Docker version", &export_data.docker_info.server_version);

        Ok(())
    }
}

impl Default for ImportCommand {
    fn default() -> Self {
        Self::new()
    }
}
