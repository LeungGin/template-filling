use serde_json::json;
use template_filling::fill;

#[test]
fn test_space() {
    let data = json!({
        "arrays": [1, 2, 3]
    });
    assert_eq!(
        fill(r#"{%for i in arrays%}{{i}}{%endfor%}"#, Some(&data)),
        "123"
    );
    assert_eq!(
        fill(r#"{% for i in arrays%}{{i}}{%endfor%}"#, Some(&data)),
        "123"
    );
    assert_eq!(
        fill(r#"{%for i in arrays %}{{i}}{%endfor%}"#, Some(&data)),
        "123"
    );
    assert_eq!(
        fill(r#"{% for i in arrays %}{{i}}{%endfor%}"#, Some(&data)),
        "123"
    );
    assert_eq!(
        fill(r#"{%for i in arrays%}{{i}}{% endfor %}"#, Some(&data)),
        "123"
    );
    assert_eq!(
        fill(r#"{%for i in arrays   %}{{i}}{%endfor%}"#, Some(&data)),
        "123"
    );
    assert_eq!(
        fill(r#"{%   for i in arrays%}{{i}}{%endfor%}"#, Some(&data)),
        "123"
    );
    assert_eq!(
        fill(r#"{%  for  i  in  arrays   %}{{i}}{%endfor%}"#, Some(&data)),
        "123"
    );
}

#[test]
fn test_join_with() {
    let data = json!({
        "arrays": [1, 2, 3]
    });
    assert_eq!(
        fill(
            r#"{% for i in arrays %}{$ join_with = , $}{{ i }}{% endfor %}"#,
            Some(&data)
        ),
        "1,2,3"
    );
    assert_eq!(
        fill(
            r#"{% for i in arrays %}{$ join_with = , $}
{{ i }}
{% endfor %}"#,
            Some(&data)
        ),
        "1,2,3"
    );
    assert_eq!(
        fill(
            r#"{% for i in arrays %}
{$ join_with = , $}
{{ i }}
{% endfor %}"#,
            Some(&data)
        ),
        "1,2,3"
    );
    assert_eq!(
        fill(
            r#"{% for i in arrays %}
    {$ join_with = , $}
    {{ i }}
{% endfor %}"#,
            Some(&data)
        ),
        "1,2,3"
    );
    assert_eq!(
        fill(
            r#"{% for i in arrays %}{$ join_with = \n $}{{ i }}{% endfor %}"#,
            Some(&data)
        ),
        "1\n2\n3"
    );
    assert_eq!(
        fill(
            r#"{% for i in arrays %}
    {$ join_with = \n $}
    {{ i }}
{% endfor %}"#,
            Some(&data)
        ),
        "1\n2\n3"
    );
    assert_eq!(
        fill(
            r#"{% for i in arrays %}{$ join_with = 分隔 $}{{ i }}{% endfor %}"#,
            Some(&data)
        ),
        "1分隔2分隔3"
    );
}
