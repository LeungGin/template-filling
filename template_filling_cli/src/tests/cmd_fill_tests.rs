use crate::fill;

#[test]
fn test() {
    // // 重定向 stdout 到内存缓冲区
    // let mut buffer = Vec::new();
    // io::set_output(Box::new(&mut buffer)); // 替换全局 stdout

    // 调用函数
    fill(
        "src/tests/cmd_fill_tests_template.tmpl".to_owned(),
        Some("{\"test_fill_data_attr\":\"abc\"}".to_owned()),
        None,
        None,
    );

    // // 恢复原有 stdout（避免影响其他测试）
    // io::set_output(Box::new(io::stdout()));

    // // 验证输出
    // assert_eq!(String::from_utf8(buffer).unwrap().trim(), "Hello, world!");
}
