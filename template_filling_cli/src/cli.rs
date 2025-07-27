use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Fill the template
    Fill {
        /// Template path
        #[arg(short = 'p', long = "template")]
        template_path_str: String,
        /// Data string (Json)
        #[arg(short = 'd', long = "data")]
        data_str: Option<String>,
        /// Data file path
        #[arg(short = 'f', long = "data_file")]
        data_path_str: Option<String>,
        /// Filling result output file path
        #[arg(short = 'o', long = "output")]
        output_path_str: Option<String>,
    },
    /// Batch fill the template
    BatchFill {
        /// Template directory path
        #[arg(short = 'p', long = "template_dir")]
        template_directory_path_str: String,
        /// Tag name of the template which will be loaded
        /// For Example, tag is 'xxx', and file 'xxx_file_name.tmpl' will be loaded
        #[arg(short = 't', long = "tag")]
        template_tag: Option<String>,
        /// Data string (Json)
        #[arg(short = 'd', long = "data")]
        data_str: Option<String>,
        /// Data file path
        #[arg(short = 'f', long = "data_file")]
        data_path_str: Option<String>,
        /// Filling result output directory path
        #[arg(short = 'o', long = "output")]
        output_directory_path_str: Option<String>,
        /// Turn off the default loading of Json file with the same name as the template as data input
        #[arg(long = "disable_same_name_date_file")]
        disable_same_name_date_file: bool,
    },
    /// Print version
    #[command(alias = "v")]
    Version,
}
