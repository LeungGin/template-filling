use std::{
    fs,
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use cli::{Cli, Command};
use serde_json::Value;

#[cfg(test)]
mod tests;

mod cli;

const TEMPLATE_SUFFIX: &str = ".template";
const TEMPLATE_SUFFIX_SHORT: &str = ".tmpl";

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Command::Fill {
            template_path_str,
            data_str,
            data_path_str,
            output_path_str,
        } => fill(template_path_str, data_str, data_path_str, output_path_str),
        Command::BatchFill {
            template_directory_path_str,
            template_tag,
            data_str,
            data_path_str,
            output_directory_path_str,
            disable_same_name_date_file,
        } => batch_fill(
            template_directory_path_str,
            template_tag,
            data_str,
            data_path_str,
            output_directory_path_str,
            disable_same_name_date_file,
        ),
        Command::Version => version(),
    }
}

struct Template {
    pub path_str: String,
    pub output_path_str: Option<String>,
    pub version: Option<String>,
}

impl Template {
    fn get_content(&self) -> String {
        fs::read_to_string(self.path_str.clone()).expect("Read template fail")
    }

    fn get_same_name_data_file_value(&self) -> Option<Value> {
        let path_str = format!(
            "{}.json",
            match self.path_str.ends_with(TEMPLATE_SUFFIX_SHORT) {
                true => &self.path_str[0..self.path_str.len() - TEMPLATE_SUFFIX_SHORT.len()],
                false => &self.path_str[0..self.path_str.len() - TEMPLATE_SUFFIX.len()],
            }
        );
        let content = fs::read_to_string(path_str).expect("Read same name data file fail");
        serde_json::from_str(&content).expect("Parse same name file content fail")
    }
}

fn fill(
    template_path_str: String,
    data_str: Option<String>,
    data_path_str: Option<String>,
    output_path_str: Option<String>,
) {
    let template_path = Path::new(&template_path_str);
    let template = package_template(&template_path, &None, &output_path_str);
    let data = load_data(&data_str, &data_path_str);
    fill_0(&template, &data);
}

fn fill_0(template: &Template, data: &Option<Value>) {
    // Filling
    let template_content = template.get_content();
    let filled = if cfg!(debug_assertions) && cfg!(not(test)) {
        let start = Instant::now();
        let filled = template_filling::fill(template_content, data.as_ref());
        let elapsed = start.elapsed();
        println!("[debug] fill::fill_template time elapsed is {:?}", elapsed);
        if elapsed.as_millis() >= 5 {
            panic!("The execution time of template_filling::fill has exceeded the 5ms performance threshold")
        }
        filled
    } else {
        template_filling::fill(template_content, data.as_ref())
    };
    // Output or print result
    if let Some(output_path_str) = template.output_path_str.as_ref() {
        println!("Output filled result to {}", output_path_str);
        // Create output path parent
        let output_path = Path::new(&output_path_str);
        if let Some(parent_path) = output_path.parent() {
            if !fs::exists(parent_path).expect("Check parent of output path exists fail") {
                fs::create_dir_all(parent_path).expect(
                    "Create all of output path parent components if they are missing, but fail",
                );
            }
        }
        // Write output
        fs::write(output_path, filled).expect("Output filled result fail");
    } else {
        println!("Filled result:\n{}", filled);
    }
}

fn load_data(data_str: &Option<String>, data_path_str: &Option<String>) -> Option<Value> {
    if let Some(data_str) = data_str {
        serde_json::from_str(&data_str).expect("Parse data content fail")
    } else if let Some(data_path_str) = data_path_str {
        let data_path = Path::new(&data_path_str);
        let data_content = fs::read_to_string(&data_path).expect("Read data fail");
        serde_json::from_str(&data_content).expect("Parse data content fail")
    } else {
        None
    }
}

fn batch_fill(
    template_directory_path_str: String,
    template_tag: Option<String>,
    data_str: Option<String>,
    data_path_str: Option<String>,
    output_directory_path_str: Option<String>,
    disable_same_name_date_file: bool,
) {
    // Find available templates
    let template_directory_path = Path::new(&template_directory_path_str);
    let templates = find_all_available_templates(
        template_directory_path,
        &template_tag,
        &output_directory_path_str,
    );
    if templates.is_none() {
        println!(
            "No more template be found in {}",
            template_directory_path_str
        );
        return;
    }
    // Load data
    let mut data = load_data(&data_str, &data_path_str);
    // Loop available templates
    for template in templates.unwrap() {
        println!(
            "Filling: ({}) {}",
            template.version.as_ref().map_or("None", |v| v),
            template.path_str
        );
        // If data is none, load same name data file
        if data.is_none() && !disable_same_name_date_file {
            data = template.get_same_name_data_file_value();
        }
        fill_0(&template, &data);
    }
}

fn find_all_available_templates(
    template_directory_path: &Path,
    template_tag: &Option<String>,
    output_directory_path_str: &Option<String>,
) -> Option<Vec<Template>> {
    let mut matches = Vec::new();

    if template_directory_path.is_dir() {
        let template_prefix = template_tag.clone().map(|tag| tag.clone() + "_");
        for entry in fs::read_dir(template_directory_path).expect("Read template directory fail") {
            let entry = entry.expect("Loop read template directory fail");
            let entry_path = entry.path();
            if entry_path.is_dir() {
                if let Some(mut templates) = find_all_available_templates(
                    &entry_path,
                    template_tag,
                    output_directory_path_str,
                ) {
                    matches.append(&mut templates);
                }
            } else {
                let entry_name = entry
                    .file_name()
                    .into_string()
                    .expect("Get template name fail");
                if (template_prefix.is_none()
                    || entry_name.starts_with(template_prefix.as_ref().unwrap()))
                    && (entry_name.ends_with(TEMPLATE_SUFFIX)
                        || entry_name.ends_with(TEMPLATE_SUFFIX_SHORT))
                {
                    let template =
                        package_template(&entry_path, template_tag, output_directory_path_str);
                    matches.push(template);
                }
            }
        }
    }

    if matches.is_empty() {
        None
    } else {
        Some(matches)
    }
}

fn package_template(
    template_path: &Path,
    template_tag: &Option<String>,
    output_directory_path_str: &Option<String>,
) -> Template {
    let template_name = template_path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    // tagname_@p_template_name_1.sql_@v_1.0.tmpl
    let part_position = find_unicode_pos(&template_name, "_@p_");
    let version_position = find_unicode_pos(&template_name, "_@v_");

    let output_file_name = if let Some(p_pos) = part_position {
        if p_pos + 7 + 1 < template_name.len() {
            if let Some(v_pos) = version_position {
                if p_pos + 7 + 1 < v_pos {
                    template_name[p_pos + 8..v_pos].to_owned()
                } else {
                    generate_random_file_name(template_tag)
                }
            } else {
                template_name[p_pos + 8..].to_owned()
            }
        } else {
            generate_random_file_name(template_tag)
        }
    } else {
        generate_random_file_name(template_tag)
    };

    let version = if let Some(v_pos) = version_position {
        if (part_position.is_some() && v_pos > part_position.unwrap() || part_position.is_none())
            && v_pos + 4 + 1 < template_name.len()
        {
            Some(template_name[v_pos + 4 + 1..].to_owned())
        } else {
            None
        }
    } else {
        None
    };

    Template {
        path_str: template_path.to_str().unwrap().to_owned(),
        output_path_str: if let Some(output_directory) = output_directory_path_str {
            Some(format!(
                "{}{}{}",
                output_directory,
                std::path::MAIN_SEPARATOR,
                output_file_name
            ))
        } else {
            None
        },
        version,
    }
}

fn generate_random_file_name(prefix_name: &Option<String>) -> String {
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    match prefix_name {
        Some(prefix_name) => format!("{}_{}", prefix_name, timestamp_ns),
        None => format!("output_{}", timestamp_ns),
    }
}

fn find_unicode_pos(text: &str, pattern: &str) -> Option<usize> {
    text.match_indices(pattern)
        .next()
        .map(|(byte_pos, _)| text[..byte_pos].chars().count())
}

fn version() {
    println!("{}", env!("CARGO_PKG_VERSION"));
}
