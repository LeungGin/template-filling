use serde_json::json;
use template_filling::fill;

#[test]
fn test_space() {
    assert_eq!(
        fill(r#"{$aaa=1$}{% if aaa == 1 %}Pass{% endif %}"#, None),
        "Pass"
    );
    assert_eq!(
        fill(r#"{$ aaa=1$}{% if aaa == 1 %}Pass{% endif %}"#, None),
        "Pass"
    );
    assert_eq!(
        fill(r#"{$aaa=1 $}{% if aaa == 1 %}Pass{% endif %}"#, None),
        "Pass"
    );
    assert_eq!(
        fill(r#"{$ aaa=1 $}{% if aaa == 1 %}Pass{% endif %}"#, None),
        "Pass"
    );
    assert_eq!(
        fill(r#"{$aaa =1$}{% if aaa == 1 %}Pass{% endif %}"#, None),
        "Pass"
    );
    assert_eq!(
        fill(r#"{$aaa= 1$}{% if aaa == 1 %}Pass{% endif %}"#, None),
        "Pass"
    );
    assert_eq!(
        fill(r#"{$aaa = 1$}{% if aaa == 1 %}Pass{% endif %}"#, None),
        "Pass"
    );
    assert_eq!(
        fill(r#"{$ aaa = 1 $}{% if aaa == 1 %}Pass{% endif %}"#, None),
        "Pass"
    );
}

#[test]
fn test_set_env() {
    let data = json!({
        "attr_bool": false, "attr_num": 0, "attr_num_2": 222, "attr_str": "", "attr_unicode": "非中文"
    });
    assert_eq!(
        fill(
            r#"{$ attr_bool = true $}{% if attr_bool == true %}Pass{% endif %}"#,
            Some(&data)
        ),
        "Pass"
    );
    assert_eq!(
        fill(
            r#"{$ attr_num = 1 $}{% if attr_num == 1 %}Pass{% endif %}"#,
            Some(&data)
        ),
        "Pass"
    );
    assert_eq!(
        fill(
            r#"{$ attr_str = "123" $}{% if attr_str == "123" %}Pass{% endif %}"#,
            Some(&data)
        ),
        "Pass"
    );
    assert_eq!(
        fill(
            r#"{$ attr_unicode = "中文" $}{% if attr_unicode == "中文" %}Pass{% endif %}"#,
            Some(&data)
        ),
        "Pass"
    );
    assert_eq!(
        fill(
            r#"{$ attr_num = attr_num_2 $}{% if attr_num == 222 %}Pass{% endif %}"#,
            Some(&data)
        ),
        "Pass"
    );
}
