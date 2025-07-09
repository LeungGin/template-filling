use serde_json::json;
use template_filling::fill;

#[test]
fn test_space() {
    assert_eq!(fill(r#"{%if 1 == 1%}Pass{%endif%}"#, None), "Pass");
    assert_eq!(fill(r#"{% if 1 == 1%}Pass{%endif%}"#, None), "Pass");
    assert_eq!(fill(r#"{%if 1 == 1 %}Pass{%endif%}"#, None), "Pass");
    assert_eq!(fill(r#"{%if 1 == 1%}Pass{% endif%}"#, None), "Pass");
    assert_eq!(fill(r#"{%if 1 == 1%}Pass{%endif %}"#, None), "Pass");
    assert_eq!(fill(r#"{% if 1 == 1 %}Pass{%endif%}"#, None), "Pass");
    assert_eq!(fill(r#"{%if 1 == 1%}Pass{% endif %}"#, None), "Pass");
    assert_eq!(fill(r#"{% if 1 == 1 %}Pass{% endif %}"#, None), "Pass");
}

#[test]
fn test_equal() {
    assert_eq!(fill(r#"{% if true %}Pass{% endif %}"#, None), "Pass");
    assert_eq!(
        fill(r#"{% if true == true %}Pass{% endif %}"#, None),
        "Pass"
    );
    assert_eq!(
        fill(r#"{% if false == false %}Pass{% endif %}"#, None),
        "Pass"
    );
    assert_eq!(fill(r#"{% if 1 == 1 %}Pass{% endif %}"#, None), "Pass");
    assert_eq!(fill(r#"{% if 123 == 123 %}Pass{% endif %}"#, None), "Pass");
    assert_eq!(fill(r#"{% if "" == "" %}Pass{% endif %}"#, None), "Pass");
    assert_eq!(fill(r#"{% if "1" == "1" %}Pass{% endif %}"#, None), "Pass");
    assert_eq!(fill(r#"{% if "a" == "a" %}Pass{% endif %}"#, None), "Pass");
    assert_eq!(
        fill(r#"{% if "abc" == "abc" %}Pass{% endif %}"#, None),
        "Pass"
    );
    assert_eq!(
        fill(r#"{% if "中文" == "中文" %}Pass{% endif %}"#, None),
        "Pass"
    );
    let val = json!({
        "eq_l_bool": true, "eq_l_num": 123, "eq_l_str": "abc", "eq_l_unicode": "中文",
        "eq_r_bool": true, "eq_r_num": 123, "eq_r_str": "abc", "eq_r_unicode": "中文"
    });
    assert_eq!(
        fill(r#"{% if $eq_l_bool %}Pass{% endif %}"#, Some(&val)),
        "Pass"
    );
    assert_eq!(
        fill(
            r#"{% if $eq_l_bool == $eq_r_bool %}Pass{% endif %}"#,
            Some(&val)
        ),
        "Pass"
    );
    assert_eq!(
        fill(
            r#"{% if $eq_l_num == $eq_r_num %}Pass{% endif %}"#,
            Some(&val)
        ),
        "Pass"
    );
    assert_eq!(
        fill(
            r#"{% if $eq_l_str == $eq_r_str %}Pass{% endif %}"#,
            Some(&val)
        ),
        "Pass"
    );
    assert_eq!(
        fill(
            r#"{% if $eq_l_unicode == $eq_r_unicode %}Pass{% endif %}"#,
            Some(&val)
        ),
        "Pass"
    );
}

#[test]
fn test_unequal() {
    assert_eq!(fill(r#"{% if true != true %}Pass{% endif %}"#, None), "");
    assert_eq!(fill(r#"{% if false != false %}Pass{% endif %}"#, None), "");
    assert_eq!(fill(r#"{% if 1 != 1 %}Pass{% endif %}"#, None), "");
    assert_eq!(fill(r#"{% if 123 != 123 %}Pass{% endif %}"#, None), "");
    assert_eq!(fill(r#"{% if "" != "" %}Pass{% endif %}"#, None), "");
    assert_eq!(fill(r#"{% if "1" != "1" %}Pass{% endif %}"#, None), "");
    assert_eq!(fill(r#"{% if "a" != "a" %}Pass{% endif %}"#, None), "");
    assert_eq!(fill(r#"{% if "abc" != "abc" %}Pass{% endif %}"#, None), "");
    assert_eq!(
        fill(r#"{% if "中文" != "中文" %}Pass{% endif %}"#, None),
        ""
    );
    let val = json!({
        "eq_l_bool": true, "eq_l_num": 123, "eq_l_str": "abc", "eq_l_unicode": "中文",
        "eq_r_bool": true, "eq_r_num": 123, "eq_r_str": "abc", "eq_r_unicode": "中文"
    });
    assert_eq!(
        fill(
            r#"{% if $eq_l_bool != $eq_r_bool %}Pass{% endif %}"#,
            Some(&val)
        ),
        ""
    );
    assert_eq!(
        fill(
            r#"{% if $eq_l_num != $eq_r_num %}Pass{% endif %}"#,
            Some(&val)
        ),
        ""
    );
    assert_eq!(
        fill(
            r#"{% if $eq_l_str != $eq_r_str %}Pass{% endif %}"#,
            Some(&val)
        ),
        ""
    );
    assert_eq!(
        fill(
            r#"{% if $eq_l_unicode != $eq_r_unicode %}Pass{% endif %}"#,
            Some(&val)
        ),
        ""
    );
}
