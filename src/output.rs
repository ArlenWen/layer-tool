use colored::*;

/// Print a success message in green
pub fn print_success(message: &str) {
    println!("{}", message.green());
}

/// Print an error message in red
pub fn print_error(message: &str) {
    println!("{}", message.red());
}

/// Print a warning message in yellow
pub fn print_warning(message: &str) {
    println!("{}", message.yellow());
}

/// Print an info message in blue
pub fn print_info(message: &str) {
    println!("{}", message.blue());
}

/// Print a progress/status message in cyan
pub fn print_progress(message: &str) {
    println!("{}", message.cyan());
}

/// Print a header message in bold white
pub fn print_header(message: &str) {
    println!("{}", message.bold().white());
}

/// Print a value with a label (label in white, value in bright white)
pub fn print_labeled_value(label: &str, value: &str) {
    println!("{}: {}", label.white(), value.bright_white());
}

/// Print a check result with appropriate color
pub fn print_check_result(label: &str, status: &str, is_success: bool) {
    let colored_status = if is_success {
        status.green()
    } else if status.contains("Skipped") {
        status.yellow()
    } else {
        status.red()
    };
    println!("  {}: {}", label.white(), colored_status);
}

/// Print a section separator
pub fn print_separator() {
    println!("{}", "=".repeat(50).bright_black());
}

/// Print formatted file size with label
pub fn print_file_info(label: &str, path: &str, size: &str) {
    println!("{}: {}", label.white(), path.bright_white());
    println!("{}: {}", "File size".white(), size.bright_cyan());
}

/// Print container information
pub fn print_container_info(label: &str, name: &str, id: &str) {
    println!("{}: {} ({})", label.white(), name.bright_white(), id.bright_black());
}

/// Print checksum information
pub fn print_checksum(label: &str, checksum: &str) {
    println!("{}: {}", label.white(), checksum.bright_green());
}

/// Print a list item with bullet point
pub fn print_list_item(item: &str) {
    println!("  • {}", item.white());
}

/// Print warnings section header and items
pub fn print_warnings_section(warnings: &[String]) {
    if !warnings.is_empty() {
        println!("\n{} {}", "⚠".yellow(), "Warnings:".yellow().bold());
        for warning in warnings {
            println!("  {}", warning.yellow());
        }
    }
}

/// Print errors section header and items
pub fn print_errors_section(errors: &[String]) {
    if !errors.is_empty() {
        println!("\n{} {}", "❌".red(), "Errors:".red().bold());
        for error in errors {
            println!("  {}", error.red());
        }
    }
}

/// Print a section header with decorative formatting
pub fn print_section_header(title: &str) {
    println!("\n{}", format!("=== {} ===", title).bold().bright_white());
}

/// Print key-value pairs in a formatted way
pub fn print_metadata_item(key: &str, value: &str) {
    println!("  {}: {}", key.white(), value.bright_white());
}

/// Print nested metadata item (with extra indentation)
pub fn print_nested_metadata_item(key: &str, value: &str) {
    println!("    {}: {}", key.white(), value.bright_white());
}
