use template_filling::fill;

#[test]
fn test_line_feed_n() {
    assert_eq!(fill("\n123", None), "\n123");
    assert_eq!(fill("\n\n123", None), "\n\n123");
    assert_eq!(fill("123\n", None), "123\n");
    assert_eq!(fill("123\n\n", None), "123\n\n");
    assert_eq!(fill("123\n123", None), "123\n123");
    assert_eq!(fill("123\n123\n123", None), "123\n123\n123");
}

#[test]
fn test_line_feed_rn() {
    assert_eq!(fill("\r\n123", None), "\r\n123");
    assert_eq!(fill("\r\n\r\n123", None), "\r\n\r\n123");
    assert_eq!(fill("123\r\n", None), "123\r\n");
    assert_eq!(fill("123\r\n\r\n", None), "123\r\n\r\n");
    assert_eq!(fill("123\r\n123", None), "123\r\n123");
    assert_eq!(fill("123\r\n123\r\n123", None), "123\r\n123\r\n123");
}

#[test]
fn test_line_feed_mix() {
    assert_eq!(fill("\r\n\n123", None), "\r\n\n123");
    assert_eq!(fill("123\r\n\n", None), "123\r\n\n");
    assert_eq!(fill("123\n\r\n", None), "123\n\r\n");
    assert_eq!(fill("123\r\n123\n", None), "123\r\n123\n");
}
