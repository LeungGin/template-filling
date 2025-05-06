use std::borrow::Cow;

use chrono::Local;
use regex::{Captures, Regex};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct PlaceholderDefine {
    pub env: String,
    pub template: String,
    pub fill_type: String,
    #[serde(default)]
    pub separator: String,
}

pub fn fill_template(template_content: String, data: &Value) -> String {
    // syntax mark
    let template_content = syntax_mark(&template_content);
    // replace all ${}
    let replaced = fill_template_0(&template_content, data, data);
    // replace all ${{}}
    let regex = Regex::new(r"\$\{(\{[\s\S]*?\})\}").expect("Create placeholder S{{}} regex fail");
    let replaced = regex.replace_all(&replaced, |caps: &Captures<'_>| {
        let placeholder_define_json = caps.get(1).unwrap().as_str();
        let placeholder_define: PlaceholderDefine = serde_json::from_str(placeholder_define_json)
            .expect("Parse placeholder define json fail");

        if placeholder_define.fill_type == "loop" {
            let loop_data = data.get(placeholder_define.env);
            if let Some(loop_data) = loop_data {
                if loop_data.is_array() {
                    let loop_items = loop_data.as_array().unwrap();
                    let mut loop_replaced_items: Vec<String> = Vec::with_capacity(loop_items.len());
                    for item_data in loop_items {
                        let item_replaced =
                            fill_template_0(&placeholder_define.template, data, item_data);
                        loop_replaced_items.push(item_replaced.to_string());
                    }
                    loop_replaced_items.join(&placeholder_define.separator)
                } else {
                    caps.get(0).unwrap().as_str().to_owned()
                }
            } else {
                caps.get(0).unwrap().as_str().to_owned()
            }
        } else {
            // fill_type == 'value'
            let local_data = data.get(placeholder_define.env);
            if let Some(local_data) = local_data {
                fill_template_0(&placeholder_define.template, data, local_data).to_string()
            } else {
                caps.get(0).unwrap().as_str().to_owned()
            }
        }
    });
    replaced.to_string()
}

fn syntax_mark(template_content: &String) -> String {
    // for-in
    let regex = Regex::new(r"\{\%\s*for\s+(.+?)\s+in\s+(.+?)\%\}([\s\S]*?)\{\%\s*endfor\s*\%\}")
        .expect("Create placeholder {%for in%}{%endfor%} regex fail");
    let marked = regex.replace_all(&template_content, |caps: &Captures<'_>| {
        println!("0 = {}", caps.get(0).unwrap().as_str().to_owned());
        println!("1 = {}", caps.get(1).unwrap().as_str().to_owned());
        println!("2 = {}", caps.get(2).unwrap().as_str().to_owned());
        caps.get(0).unwrap().as_str().to_owned()
    });
    template_content.to_owned()
}

fn fill_template_0<'a>(
    template_content: &'a String,
    global_data: &'a Value,
    local_data: &'a Value,
) -> Cow<'a, str> {
    let regex = Regex::new(r"\{\{([\s\S]*?)\}\}").expect("Create placeholder {{}} regex fail");
    regex.replace_all(&template_content, |caps: &Captures<'_>| {
        let placeholder = caps.get(1).unwrap().as_str();
        if placeholder.starts_with("@") {
            get_system_env(placeholder).unwrap_or(caps.get(0).unwrap().as_str().to_owned())
        } else {
            if placeholder.starts_with(".") {
                // local env
                get_by_step_in_key(local_data, placeholder)
                    .unwrap_or(caps.get(0).unwrap().as_str().to_owned())
            } else {
                // global env
                get_by_step_in_key(global_data, placeholder)
                    .unwrap_or(caps.get(0).unwrap().as_str().to_owned())
            }
        }
    })
}

fn get_system_env(env: &str) -> Option<String> {
    if env == "@now" {
        return Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
    }
    None
}

fn get_by_step_in_key(mut data: &Value, key: &str) -> Option<String> {
    if !key.contains(".") {
        return data.get(key).map_or(None, |v| Some(to_pure_string(v)));
    }
    let keys: Vec<&str> = key.split(".").filter(|&x| !x.is_empty()).collect();
    for k in keys {
        if !data.is_object() {
            return None;
        }
        let v = data.get(k);
        if v.is_none() {
            return None;
        }
        data = v.unwrap();
    }
    Some(to_pure_string(data))
}

fn to_pure_string(v: &Value) -> String {
    if v.is_string() {
        v.as_str().unwrap().to_owned()
    } else {
        v.to_string()
    }
}
