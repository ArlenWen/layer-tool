use anyhow::Result;
use clap::{Parser, Subcommand};
use layer_tool::{CheckCommand, CheckOptions, ExportCommand, ImportCommand};

#[derive(Parser)]
#[command(name = "layer-tool")]
#[command(about = "A tool for exporting, importing, and checking Docker container layers")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Export container layer and metadata to a file
    Export {
        /// Container ID or name to export
        container_id: String,
        /// Output file path
        output_file: String,
        /// Compress the output file using gzip
        #[arg(long)]
        compress: bool,
    },
    /// Import layer data from export file to container
    Import {
        /// Input export file path
        input_file: String,
        /// Target container ID or name
        container_id: String,
        /// Skip backing up existing layer before import
        #[arg(long)]
        no_backup: bool,
    },
    /// Check export file integrity and compatibility
    Check {
        /// Input export file path to check
        input_file: String,
        /// Skip image SHA256 verification
        #[arg(long)]
        skip_image: bool,
        /// Skip storage driver compatibility check
        #[arg(long)]
        skip_storage: bool,
        /// Skip operating system compatibility check
        #[arg(long)]
        skip_os: bool,
        /// Skip architecture compatibility check
        #[arg(long)]
        skip_arch: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Export {
            container_id,
            output_file,
            compress,
        } => {
            let export_cmd = ExportCommand::new();
            export_cmd.execute(&container_id, &output_file, compress)?;
        }
        Commands::Import {
            input_file,
            container_id,
            no_backup,
        } => {
            let import_cmd = ImportCommand::new();
            import_cmd.execute(&input_file, &container_id, !no_backup)?;
        }
        Commands::Check {
            input_file,
            skip_image,
            skip_storage,
            skip_os,
            skip_arch,
        } => {
            let check_options = CheckOptions {
                skip_image,
                skip_storage,
                skip_os,
                skip_arch,
            };
            let check_cmd = CheckCommand::new();
            check_cmd.execute(&input_file, check_options)?;
        }
    }

    Ok(())
}
