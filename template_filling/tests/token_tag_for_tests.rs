use serde_json::json;
use template_filling::fill;

#[test]
fn test_space() {
    let val = json!({
        "arrays": [1, 2, 3]
    });
    assert_eq!(
        fill(r#"{%for i in $arrays%}{{$i}}{%endfor%}"#, Some(&val)),
        "123"
    );
    assert_eq!(
        fill(r#"{% for i in $arrays%}{{$i}}{%endfor%}"#, Some(&val)),
        "123"
    );
    assert_eq!(
        fill(r#"{%for i in $arrays %}{{$i}}{%endfor%}"#, Some(&val)),
        "123"
    );
    assert_eq!(
        fill(r#"{% for i in $arrays %}{{$i}}{%endfor%}"#, Some(&val)),
        "123"
    );
    assert_eq!(
        fill(r#"{%for i in $arrays%}{{$i}}{% endfor %}"#, Some(&val)),
        "123"
    );
    assert_eq!(
        fill(r#"{%for i in $arrays   %}{{$i}}{%endfor%}"#, Some(&val)),
        "123"
    );
    assert_eq!(
        fill(r#"{%   for i in $arrays%}{{$i}}{%endfor%}"#, Some(&val)),
        "123"
    );
    assert_eq!(
        fill(
            r#"{%  for  i  in  $arrays   %}{{$i}}{%endfor%}"#,
            Some(&val)
        ),
        "123"
    );
}

#[test]
fn test_join_with() {
    let val = json!({
        "arrays": [1, 2, 3]
    });
    assert_eq!(
        fill(
            r#"{% for i in $arrays %}{$ join_with = , $}{{ $i }}{% endfor %}"#,
            Some(&val)
        ),
        "1,2,3"
    );
    assert_eq!(
        fill(
            r#"{% for i in $arrays %}
{{ $i }}
{% endfor %}"#,
            Some(&val)
        ),
        "1,2,3"
    );
    assert_eq!(
        fill(
            r#"{% for i in $arrays %}
    {{ $i }}
{% endfor %}"#,
            Some(&val)
        ),
        "1,2,3"
    );
    assert_eq!(
        fill(
            r#"{% for i in $arrays %}{$ join_with = \n $}{{ $i }}{% endfor %}"#,
            Some(&val)
        ),
        "1\n2\n3"
    );
    assert_eq!(
        fill(
            r#"{% for i in $arrays %}
    {$ join_with = \n $}
    {{ $i }}
{% endfor %}"#,
            Some(&val)
        ),
        "1\n2\n3"
    );
    assert_eq!(
        fill(
            r#"{% for i in $arrays %}{$ join_with = 分隔 $}{{ $i }}{% endfor %}"#,
            Some(&val)
        ),
        "1分隔2分隔3"
    );
}
