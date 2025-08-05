# layer-tool

[中文](./README_zh.md)

A command-line tool for exporting, importing, and checking Docker container layers.

## Features

- **Export**: Export Docker container's read-write layer, metadata, and Docker info to a file
- **Import**: Import exported file back to an existing container's read-write layer
- **Check**: Validate exported file integrity and compatibility

## Installation

### Build from source:

```bash
cargo build --release
```

The binary will be available at `target/release/layer-tool`.

### Build static by musl:

```bash
cargo build --release --target=x86_64-unknown-linux-musl
```

The binary will be available at `target/x86_64-unknown-linux-musl/release/layer-tool`.

## Usage

### Export Container Layer

Export a container's read-write layer and metadata to a file:

```bash
layer-tool export <container_id> <output_file> [--compress]
```

**Examples:**
```bash
# Export container to uncompressed file
layer-tool export my-container container-export.tar

# Export container to compressed file
layer-tool export my-container container-export.tar.gz --compress
```

### Import Container Layer

Import layer data from an export file to an existing container:

```bash
layer-tool import <input_file> <container_id> [--no-backup]
```

**Options:**
- `--no-backup`: Skip backing up existing layer before import (WARNING: This will permanently remove existing layer data)

**Examples:**
```bash
# Import from uncompressed file (with backup)
layer-tool import container-export.tar target-container

# Import from compressed file (automatically detected)
layer-tool import container-export.tar.gz target-container

# Import without backing up existing layer
layer-tool import container-export.tar target-container --no-backup
```

### Check Export File

Validate export file integrity and compatibility:

```bash
layer-tool check <input_file> [OPTIONS]
```

**Options:**
- `--skip-image`: Skip image SHA256 verification
- `--skip-storage`: Skip storage driver compatibility check
- `--skip-os`: Skip operating system compatibility check
- `--skip-arch`: Skip architecture compatibility check

**Examples:**
```bash
# Full check
layer-tool check container-export.tar

# Skip some compatibility checks
layer-tool check container-export.tar --skip-os --skip-arch
```

## Export File Format

The export file contains:
- Container metadata (JSON)
- Docker daemon information (JSON)
- Container's upper layer (tar archive)
- Optional gzip compression

## Requirements

- Docker daemon must be running and accessible
- Sufficient permissions to access Docker and container layer directories
- Target containers must exist for import operations

## Security Considerations

- The tool requires access to Docker daemon and container layer directories
- Export files may contain sensitive data from the container's file system
- Always validate export files before importing to production containers
- Use appropriate file permissions for export files

## Error Handling

The tool provides detailed error messages for common issues:
- Container not found
- Permission denied
- Invalid export file format
- Checksum mismatches
- Compatibility issues

## Limitations

- Currently supports overlay2 storage driver
- Requires Docker CLI to be available
- Does not handle running containers (stop container before export/import)
- Limited to Linux systems

## How it Works

### Export Process
1. Gather container metadata and Docker daemon information
2. Locate the container's read-write layer directory (upper directory)
3. Create a tar archive of the layer data
4. Calculate checksums to ensure integrity
5. Package metadata, Docker info, and layer data together
6. Optionally compress the final file

### Import Process
1. Read and validate the export file
2. Extract metadata and Docker information
3. Decompress if needed
4. Backup the target container's existing layer (if it exists and is not empty, unless --no-backup is specified)
5. Extract layer data to the target container's upper directory
6. Verify checksums of the imported data

### Check Process
1. Validate file structure and format
2. Check metadata integrity
3. Verify layer archive readability
4. Perform compatibility checks with current Docker environment
5. Generate detailed validation report

## Use Cases

- **Container Backup**: Backup container's read-write layer for later restoration
- **Container Migration**: Migrate container state between different environments
- **Development Environment**: Share container state among development teams
- **Testing**: Create consistent test environment snapshots
- **Disaster Recovery**: Quickly restore containers to known states

## Troubleshooting

### Common Issues

**Permission Errors**
```bash
# Ensure user is in docker group
sudo usermod -aG docker $USER
# Or run with sudo
sudo layer-tool export container-name backup.tar
```

**Container Not Found**
```bash
# Check if container exists
docker ps -a
# Use full container ID or correct name
```

**Storage Driver Incompatibility**
```bash
# Check Docker storage driver
docker info | grep "Storage Driver"
# Use --skip-storage to bypass check (if safe)
layer-tool check backup.tar --skip-storage
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License.
