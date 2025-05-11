use std::{fs, path::Path, time::Instant};

use clap::Parser;
use cli::{Cli, Command};
use serde_json::Value;

mod cli;
mod fill;

struct Template {
    pub path_string: String,
    pub export_path_string: Option<String>,
    pub version: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Command::Fill {
            template_directory_path_string,
            template_tag,
            data_path_string,
            export_directory_path_string,
        } => fill(
            template_directory_path_string,
            template_tag,
            data_path_string,
            export_directory_path_string,
        ),
        Command::Version => version(),
    }
}

fn fill(
    template_directory_path_string: String,
    template_tag: String,
    data_path_string: String,
    export_directory_path_string: Option<String>,
) {
    // find available templates
    let template_directory_path = Path::new(&template_directory_path_string);
    let available_templates = find_all_available_templates(
        template_directory_path,
        &template_tag,
        &export_directory_path_string,
    );

    if available_templates.is_none() {
        println!(
            "No more template be found in {}",
            template_directory_path_string
        )
    }

    let data_path = Path::new(&data_path_string);
    let data_content = fs::read_to_string(&data_path).expect("Read data fail");
    let data: Value = serde_json::from_str(&data_content).expect("Parse data content fail");

    // loop available templates
    for template in available_templates.unwrap() {
        println!(
            "Filling {}: {}",
            template.path_string,
            if let Some(v) = template.version {
                format!("v{}", v)
            } else {
                "No version".to_string()
            }
        );

        // fill template
        let template_path = Path::new(&template.path_string);
        let template_content = fs::read_to_string(&template_path).expect("Read template fail");
        let filled = if cfg!(debug_assertions) {
            let start = Instant::now();
            let filled = fill::fill_template(template_content, &data);
            let elapsed = start.elapsed();
            println!("[debug] fill::fill_template time elapsed is {:?}", elapsed);
            if elapsed.as_millis() >= 10 {
                panic!("The execution time of fill::fill_template has exceeded the 10ms performance threshold")
            }
            filled
        } else {
            fill::fill_template(template_content, &data)
        };

        // export or print result
        if let Some(export_path_string) = template.export_path_string {
            println!("Exporting filled result to {}", export_path_string);

            // create export path parent
            let export_path = Path::new(&export_path_string);
            if let Some(parent_path) = export_path.parent() {
                if !fs::exists(parent_path).expect("Check parent of export path exists fail") {
                    fs::create_dir_all(parent_path).expect(
                        "Create all of export path parent components if they are missing, but fail",
                    );
                }
            }
            // write export
            fs::write(export_path, filled).expect("Export filled result fail");
        } else {
            println!("Filled result:\n{}", filled);
        }
    }
}

fn find_all_available_templates(
    template_directory_path: &Path,
    template_tag: &String,
    export_directory_path_string: &Option<String>,
) -> Option<Vec<Template>> {
    let mut matches = Vec::new();

    if template_directory_path.is_dir() {
        let template_prefix = template_tag.clone() + ".";
        for entry in fs::read_dir(template_directory_path).expect("Read template directory fail") {
            let entry = entry.expect("Loop read template directory fail");
            let entry_path = entry.path();
            if entry_path.is_dir() {
                if let Some(mut templates) = find_all_available_templates(
                    &entry_path,
                    template_tag,
                    export_directory_path_string,
                ) {
                    matches.append(&mut templates);
                }
            } else {
                let entry_name = entry
                    .file_name()
                    .into_string()
                    .expect("Get template name fail");
                if entry_name.starts_with(&template_prefix) && entry_name.ends_with(".template") {
                    matches.push(package_template_obj(
                        &entry_path,
                        template_tag,
                        export_directory_path_string,
                    ));
                }
            }
        }
    }

    if matches.is_empty() {
        return None;
    }
    Some(matches)
}

fn package_template_obj(
    template_path: &Path,
    template_tag: &String,
    export_directory_path_string: &Option<String>,
) -> Template {
    let template_name = template_path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let part_position = find_unicode_pos(&template_name, ".[part].");
    let version_position = find_unicode_pos(&template_name, ".[v].");

    let export_file_name = if let Some(p_pos) = part_position {
        if p_pos + 7 + 1 < template_name.len() {
            if let Some(v_pos) = version_position {
                if p_pos + 7 + 1 < v_pos {
                    template_name[p_pos + 8..v_pos].to_owned()
                } else {
                    template_tag.to_owned()
                }
            } else {
                template_name[p_pos + 8..].to_owned()
            }
        } else {
            template_tag.to_owned()
        }
    } else {
        template_tag.to_owned()
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
        path_string: template_path.to_str().unwrap().to_owned(),
        export_path_string: if let Some(export_directory) = export_directory_path_string {
            Some(format!(
                "{}{}{}",
                export_directory,
                std::path::MAIN_SEPARATOR,
                export_file_name
            ))
        } else {
            None
        },
        version,
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
