use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Fill the template and export the text
    Fill {
        /// Template directory path string
        #[arg(long = "tmpl_dir")]
        template_directory_path_string: String,
        /// The tag of the template to be selected
        #[arg(long = "tag")]
        template_tag: String,
        /// The data file path string
        #[arg(long = "data")]
        data_path_string: String,
        /// Result export target directory path string
        #[arg(long = "export_dir")]
        export_directory_path_string: Option<String>,
    },
    /// Get version
    Version,
}
