use template_filling::fill;

#[test]
fn test() {
    assert_eq!(fill("abc", None), "abc");
    assert_eq!(fill("1\n12\n123", None), "1\n12\n123");
    assert_eq!(fill("a\nab\nabc", None), "a\nab\nabc");
    assert_eq!(fill("一\n一二\n一二三", None), "一\n一二\n一二三");
    assert_eq!(fill("~\n~!\n~!@", None), "~\n~!\n~!@");
    assert_eq!(fill("\\\n\\r\n\\r\\", None), "\\\n\\r\n\\r\\");
}
