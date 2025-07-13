use std::{fs, path::Path};

use serde_json::Value;
use template_filling::fill;

#[test]
fn dev_test() {
    // data
    let data_path = Path::new("./tests/integration_tests_template_data.json");
    let data_content = fs::read_to_string(&data_path).expect("Read data fail");
    let data: Value = serde_json::from_str(&data_content).expect("Parse data content fail");
    // template
    let template_path = Path::new("./tests/integration_tests_template.tmpl");
    let template_content = fs::read_to_string(&template_path).expect("Read template fail");
    let filling_result = fill(template_content, Some(&data));

    println!("{}", filling_result);
}

#[test]
fn test() {
    // data
    let data_path = Path::new("./tests/integration_tests_template_data.json");
    let data_content = fs::read_to_string(&data_path).expect("Read data fail");
    let data: Value = serde_json::from_str(&data_content).expect("Parse data content fail");
    // template
    let template_path = Path::new("./tests/integration_tests_template.tmpl");
    let template_content = fs::read_to_string(&template_path).expect("Read template fail");
    let filling_result = fill(template_content, Some(&data));
    // expect
    let expect_path = Path::new("./tests/integration_tests_template_expect.sql");
    let expect_content = fs::read_to_string(&expect_path).expect("Read template expect fail");

    assert_eq!(filling_result, expect_content);
}
