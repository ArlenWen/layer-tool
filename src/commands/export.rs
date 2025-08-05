use anyhow::{Context, Result};
use chrono::Utc;
use std::fs::File;
use std::path::Path;
use tar::Builder;
use tempfile::TempDir;

use crate::docker::DockerClient;
use crate::output::*;
use crate::types::ExportData;
use crate::utils::{compress_file, create_tar_archive, format_file_size, get_file_size};

pub struct ExportCommand {
    docker_client: DockerClient,
}

impl ExportCommand {
    pub fn new() -> Self {
        Self {
            docker_client: DockerClient::new(),
        }
    }

    /// Export container layer and metadata to a file
    pub fn execute(&self, container_id: &str, output_path: &str, compress: bool) -> Result<()> {
        print_progress(&format!("Starting export of container: {}", container_id));

        // Validate container exists and is ready for layer operations
        print_progress("Validating container state...");
        self.docker_client.validate_container_for_layer_operations(container_id)
            .context("Container validation failed")?;

        // Get container metadata
        print_progress("Gathering container metadata...");
        let container_metadata = self.docker_client.get_container_metadata(container_id)
            .context("Failed to get container metadata")?;

        // Get Docker info
        print_progress("Gathering Docker daemon information...");
        let docker_info = self.docker_client.get_docker_info()
            .context("Failed to get Docker info")?;

        // Get container layer path
        print_progress("Locating container layer directory...");
        let upper_layer_path = self.docker_client.get_upper_layer_path(container_id)
            .context("Failed to get container layer path")?;

        if !upper_layer_path.exists() {
            return Err(anyhow::anyhow!(
                "Container upper layer directory not found: {:?}",
                upper_layer_path
            ));
        }

        // Create temporary directory for export files
        let temp_dir = TempDir::new()
            .context("Failed to create temporary directory")?;
        let temp_path = temp_dir.path();

        // Create tar archive of the upper layer first
        print_progress("Creating layer archive...");
        let layer_tar_path = temp_path.join("layer.tar");
        let layer_checksum = create_tar_archive(&upper_layer_path, &layer_tar_path)
            .context("Failed to create layer archive")?;

        print_checksum("Layer archive created with checksum", &layer_checksum);

        // Create export data structure with the calculated checksum
        let export_data = ExportData {
            version: "1.0".to_string(),
            created: Utc::now(),
            container_metadata,
            docker_info,
            layer_checksum: layer_checksum.clone(),
            compressed: compress,
        };

        // Write metadata to temporary file
        let metadata_path = temp_path.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&export_data)
            .context("Failed to serialize export metadata")?;
        std::fs::write(&metadata_path, metadata_json)
            .context("Failed to write metadata file")?;

        // Create final export archive
        print_progress("Creating export archive...");
        let export_tar_path = temp_path.join("export.tar");
        self.create_export_archive(&metadata_path, &layer_tar_path, &export_tar_path)
            .context("Failed to create export archive")?;

        // Handle compression and final output
        let final_output_path = Path::new(output_path);
        if compress {
            print_progress("Compressing export archive...");
            let compressed_path = if output_path.ends_with(".gz") {
                final_output_path.to_path_buf()
            } else {
                final_output_path.with_extension("tar.gz")
            };

            compress_file(&export_tar_path, &compressed_path)
                .context("Failed to compress export archive")?;

            let file_size = get_file_size(&compressed_path)?;
            print_success("Export completed successfully!");
            print_file_info("Output file", &format!("{:?}", compressed_path), &format_file_size(file_size));
        } else {
            std::fs::copy(&export_tar_path, final_output_path)
                .context("Failed to copy export archive to final location")?;

            let file_size = get_file_size(final_output_path)?;
            print_success("Export completed successfully!");
            print_file_info("Output file", &format!("{:?}", final_output_path), &format_file_size(file_size));
        }

        print_container_info("Container", &export_data.container_metadata.name, container_id);
        print_labeled_value("Image", &export_data.container_metadata.image);
        print_checksum("Layer checksum", &layer_checksum);

        Ok(())
    }

    /// Create the final export archive containing metadata and layer data
    fn create_export_archive(
        &self,
        metadata_path: &Path,
        layer_tar_path: &Path,
        output_path: &Path,
    ) -> Result<()> {
        let output_file = File::create(output_path)
            .context("Failed to create export archive file")?;
        let mut builder = Builder::new(output_file);

        // Add metadata file
        builder.append_path_with_name(metadata_path, "metadata.json")
            .context("Failed to add metadata to export archive")?;

        // Add layer tar file
        builder.append_path_with_name(layer_tar_path, "layer.tar")
            .context("Failed to add layer archive to export archive")?;

        builder.finish()
            .context("Failed to finish export archive")?;

        Ok(())
    }
}

impl Default for ExportCommand {
    fn default() -> Self {
        Self::new()
    }
}
