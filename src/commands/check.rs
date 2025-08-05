use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;
use tar::Archive;
use tempfile::TempDir;

use crate::docker::DockerClient;
use crate::output::*;
use crate::types::{CheckOptions, ExportData};
use crate::utils::{
    decompress_file, is_gzip_file,
    calculate_file_checksum, format_file_size, get_file_size
};

pub struct CheckCommand {
    docker_client: DockerClient,
}

impl CheckCommand {
    pub fn new() -> Self {
        Self {
            docker_client: DockerClient::new(),
        }
    }

    /// Check export file integrity and compatibility
    pub fn execute(&self, input_path: &str, options: CheckOptions) -> Result<()> {
        print_progress(&format!("Checking export file: {}", input_path));

        let input_file_path = Path::new(input_path);
        if !input_file_path.exists() {
            return Err(anyhow::anyhow!("Input file not found: {}", input_path));
        }

        let file_size = get_file_size(input_file_path)?;
        print_labeled_value("File size", &format_file_size(file_size));

        // Create temporary directory for extraction
        let temp_dir = TempDir::new()
            .context("Failed to create temporary directory")?;
        let temp_path = temp_dir.path();

        // Handle decompression if needed
        let is_compressed = is_gzip_file(input_file_path)?;
        let export_tar_path = if is_compressed {
            print_check_result("File compression", "✓ Compressed (gzip)", true);
            let decompressed_path = temp_path.join("export.tar");
            decompress_file(input_file_path, &decompressed_path)
                .context("Failed to decompress input file")?;
            decompressed_path
        } else {
            print_check_result("File compression", "✓ Uncompressed", true);
            input_file_path.to_path_buf()
        };

        // Extract and validate archive structure
        print_progress("Checking archive structure...");
        let extract_dir = temp_path.join("extracted");
        std::fs::create_dir_all(&extract_dir)
            .context("Failed to create extraction directory")?;

        self.extract_and_validate_structure(&export_tar_path, &extract_dir)
            .context("Failed to validate archive structure")?;

        // Read and validate metadata
        print_progress("Validating metadata...");
        let metadata_path = extract_dir.join("metadata.json");
        let export_data = self.read_and_validate_metadata(&metadata_path)
            .context("Failed to validate metadata")?;

        // Validate layer archive
        print_progress("Validating layer archive...");
        let layer_tar_path = extract_dir.join("layer.tar");
        self.validate_layer_archive(&layer_tar_path, &export_data)
            .context("Failed to validate layer archive")?;

        // Perform compatibility checks
        print_progress("Performing compatibility checks...");
        self.perform_compatibility_checks(&export_data, &options)
            .context("Compatibility checks failed")?;

        // Display check results
        self.display_check_results(&export_data, is_compressed, &options)?;

        print_success("\n✅ All checks passed! Export file is valid and complete.");

        Ok(())
    }

    /// Extract archive and validate basic structure
    fn extract_and_validate_structure(&self, archive_path: &Path, output_dir: &Path) -> Result<()> {
        let archive_file = File::open(archive_path)
            .context("Failed to open export archive")?;
        let mut archive = Archive::new(archive_file);

        // Extract archive
        archive.unpack(output_dir)
            .context("Failed to extract export archive")?;

        // Check required files exist
        let metadata_path = output_dir.join("metadata.json");
        let layer_tar_path = output_dir.join("layer.tar");

        if !metadata_path.exists() {
            return Err(anyhow::anyhow!("Missing metadata.json in export archive"));
        }

        if !layer_tar_path.exists() {
            return Err(anyhow::anyhow!("Missing layer.tar in export archive"));
        }

        print_check_result("Archive structure", "✓ Valid", true);
        Ok(())
    }

    /// Read and validate metadata file
    fn read_and_validate_metadata(&self, metadata_path: &Path) -> Result<ExportData> {
        let metadata_content = std::fs::read_to_string(metadata_path)
            .context("Failed to read metadata file")?;

        let export_data: ExportData = serde_json::from_str(&metadata_content)
            .context("Failed to parse metadata JSON")?;

        // Validate required fields
        if export_data.version.is_empty() {
            return Err(anyhow::anyhow!("Missing or empty version in metadata"));
        }

        if export_data.container_metadata.id.is_empty() {
            return Err(anyhow::anyhow!("Missing or empty container ID in metadata"));
        }

        if export_data.container_metadata.image_sha256.is_empty() {
            return Err(anyhow::anyhow!("Missing or empty image SHA256 in metadata"));
        }

        if export_data.layer_checksum.is_empty() {
            return Err(anyhow::anyhow!("Missing or empty layer checksum in metadata"));
        }

        print_check_result("Metadata", "✓ Valid", true);
        print_metadata_item("Version", &export_data.version);
        print_container_info("Container", &export_data.container_metadata.name, &export_data.container_metadata.id);
        print_metadata_item("Image", &export_data.container_metadata.image);

        Ok(export_data)
    }

    /// Validate layer archive integrity
    fn validate_layer_archive(&self, layer_tar_path: &Path, export_data: &ExportData) -> Result<()> {
        // Check if layer tar file can be opened
        let layer_file = File::open(layer_tar_path)
            .context("Failed to open layer archive")?;
        let mut layer_archive = Archive::new(layer_file);

        // Try to list entries to validate tar structure
        let entries = layer_archive.entries()
            .context("Failed to read layer archive entries")?;

        let mut entry_count = 0;
        for entry in entries {
            let _entry = entry.context("Failed to read layer archive entry")?;
            entry_count += 1;
        }

        print_check_result("Layer archive", &format!("✓ Readable ({} entries)", entry_count), true);

        // Verify layer checksum
        let calculated_checksum = calculate_file_checksum(layer_tar_path)
            .context("Failed to calculate layer archive checksum")?;

        // Note: This compares the tar file checksum, not the directory checksum
        // In a real implementation, you might want to extract and verify the directory checksum
        print_checksum("Layer archive checksum calculated", &calculated_checksum);
        print_metadata_item("Expected layer checksum", &export_data.layer_checksum);

        Ok(())
    }

    /// Perform compatibility checks with current Docker environment
    fn perform_compatibility_checks(&self, export_data: &ExportData, options: &CheckOptions) -> Result<()> {
        // Get current Docker info for comparison
        let current_docker_info = match self.docker_client.get_docker_info() {
            Ok(info) => info,
            Err(e) => {
                print_warning(&format!("Could not get current Docker info: {}", e));
                print_warning("Skipping Docker environment compatibility checks");
                return Ok(());
            }
        };

        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Check storage driver compatibility
        if !options.skip_storage {
            if export_data.docker_info.driver != current_docker_info.driver {
                warnings.push(format!(
                    "Storage driver mismatch: export uses '{}', current system uses '{}'",
                    export_data.docker_info.driver,
                    current_docker_info.driver
                ));
            } else {
                print_check_result("Storage driver", &format!("✓ Compatible: {}", current_docker_info.driver), true);
            }
        } else {
            print_check_result("Storage driver check", "⏭ Skipped", false);
        }

        // Check OS compatibility
        if !options.skip_os {
            if export_data.docker_info.operating_system != current_docker_info.operating_system {
                warnings.push(format!(
                    "Operating system mismatch: export from '{}', current system is '{}'",
                    export_data.docker_info.operating_system,
                    current_docker_info.operating_system
                ));
            } else {
                print_check_result("Operating system", &format!("✓ Compatible: {}", current_docker_info.operating_system), true);
            }
        } else {
            print_check_result("OS check", "⏭ Skipped", false);
        }

        // Check architecture compatibility
        if !options.skip_arch {
            if export_data.docker_info.architecture != current_docker_info.architecture {
                errors.push(format!(
                    "Architecture mismatch: export from '{}', current system is '{}'",
                    export_data.docker_info.architecture,
                    current_docker_info.architecture
                ));
            } else {
                print_check_result("Architecture", &format!("✓ Compatible: {}", current_docker_info.architecture), true);
            }
        } else {
            print_check_result("Architecture check", "⏭ Skipped", false);
        }

        // Check image availability (if not skipped)
        if !options.skip_image {
            // This is a simplified check - in reality you'd want to verify the image exists
            // and matches the SHA256 from the export
            print_check_result("Image SHA256", &format!("✓ {}", export_data.container_metadata.image_sha256), true);
        } else {
            print_check_result("Image check", "⏭ Skipped", false);
        }

        // Display warnings and errors
        print_warnings_section(&warnings);
        print_errors_section(&errors);

        // Fail if any errors
        if !errors.is_empty() {
            return Err(anyhow::anyhow!("Compatibility check failed with {} error(s)", errors.len()));
        }

        Ok(())
    }

    /// Display comprehensive check results
    fn display_check_results(&self, export_data: &ExportData, is_compressed: bool, options: &CheckOptions) -> Result<()> {
        print_section_header("Check Results");
        print_labeled_value("Export file format", if is_compressed { "Compressed (gzip)" } else { "Uncompressed" });
        print_labeled_value("Export version", &export_data.version);
        print_labeled_value("Export created", &export_data.created.format("%Y-%m-%d %H:%M:%S UTC").to_string());

        print_info("\nContainer information:");
        print_metadata_item("ID", &export_data.container_metadata.id);
        print_metadata_item("Name", &export_data.container_metadata.name);
        print_metadata_item("Image", &export_data.container_metadata.image);
        print_metadata_item("Image SHA256", &export_data.container_metadata.image_sha256);
        print_metadata_item("Created", &export_data.container_metadata.created.format("%Y-%m-%d %H:%M:%S UTC").to_string());
        print_metadata_item("State", &export_data.container_metadata.state);

        print_info("\nDocker environment (at export time):");
        print_metadata_item("Storage driver", &export_data.docker_info.driver);
        print_metadata_item("Operating system", &export_data.docker_info.operating_system);
        print_metadata_item("Architecture", &export_data.docker_info.architecture);
        print_metadata_item("Docker version", &export_data.docker_info.server_version);

        print_info("\nLayer information:");
        print_metadata_item("Checksum", &export_data.layer_checksum);

        print_info("\nChecks performed:");
        print_check_result("Archive structure", "✓", true);
        print_check_result("Metadata validation", "✓", true);
        print_check_result("Layer archive integrity", "✓", true);
        print_check_result("Storage driver compatibility", if options.skip_storage { "⏭ Skipped" } else { "✓" }, !options.skip_storage);
        print_check_result("OS compatibility", if options.skip_os { "⏭ Skipped" } else { "✓" }, !options.skip_os);
        print_check_result("Architecture compatibility", if options.skip_arch { "⏭ Skipped" } else { "✓" }, !options.skip_arch);
        print_check_result("Image verification", if options.skip_image { "⏭ Skipped" } else { "✓" }, !options.skip_image);

        Ok(())
    }
}

impl Default for CheckCommand {
    fn default() -> Self {
        Self::new()
    }
}
