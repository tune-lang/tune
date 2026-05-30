#[test]
fn formats_basic_declarations_and_blocks() {
    let source = r#"pub   let add(a:Int,b:Int):Int={let value=a+b;value}"#;
    let formatted = tune_fmt::format_source(source);

    assert_eq!(
        formatted,
        r#"pub let add(a: Int, b: Int): Int = {
  let value = a + b
  value
}
"#
    );
}

#[test]
fn preserves_comments_and_string_literals() {
    let source = "let text=\"a  b\" -- keep\n-/ block /-\nlet next=1";
    let formatted = tune_fmt::format_source(source);

    assert!(formatted.contains("\"a  b\" -- keep"));
    assert!(formatted.contains("-/ block /-"));
    assert!(formatted.ends_with("let next = 1\n"));
}
