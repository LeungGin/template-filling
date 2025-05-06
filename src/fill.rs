use std::str;

use serde_json::Value;

pub fn fill_template(template_content: String, data: &Value) -> String {
    // Generate tokens
    let bytes = template_content.as_bytes();
    let tokens = generate_tokens(bytes);
    // debug
    println!("{:?}", tokens);
    // Fill with token
    fill(bytes, tokens, data)
}

#[derive(Debug)]
enum Symbol {
    Logical,
    Env,
    Placeholder,
}

#[derive(Debug)]
enum Token<'a> {
    Text(usize, usize),
    Placeholder(usize, usize),
    Env(usize, usize),
    Tag(Tag<'a>, Vec<Token<'a>>),
}

fn generate_tokens(template_content_bytes: &[u8]) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut head_symbol_stack: Vec<(Symbol, usize)> = Vec::with_capacity(1);
    let mut tag_token_stack: Vec<Token> = Vec::new();
    let mut last_pos = 0;

    let bytes = template_content_bytes;
    let mut i = 0;
    let len = bytes.len().saturating_sub(1);
    while i < len {
        match (&bytes[i], &bytes[i + 1]) {
            (b'{', b'%') => {
                if last_pos < i {
                    tokens.push(Token::Text(last_pos, i));
                    last_pos = i;
                }
                head_symbol_stack.push((Symbol::Logical, i + 2));
                last_pos += 2;
                i += 2;
            }
            (b'%', b'}') => {
                if let Some((Symbol::Logical, start_idx)) = head_symbol_stack.pop() {
                    let tag = generate_tag(&bytes[start_idx..i]);
                    match tag {
                        Tag::For(_, _) => {
                            tag_token_stack.push(Token::Tag(tag, Vec::new()));
                        }
                        Tag::EndFor => {
                            // TODO sub_tokens未正确赋值
                            if let Some(Token::Tag(head_tag, mut sub_tokens)) =
                                tag_token_stack.pop()
                            {
                                match head_tag {
                                    Tag::For(_, _) => {
                                        sub_tokens.push(Token::Text(last_pos, i));
                                        last_pos = i;
                                        if let Some(Token::Tag(
                                            parent_head_tag,
                                            mut parent_sub_tokens,
                                        )) = tag_token_stack.pop()
                                        {
                                            parent_sub_tokens
                                                .push(Token::Tag(head_tag, sub_tokens));
                                            tag_token_stack.push(Token::Tag(
                                                parent_head_tag,
                                                parent_sub_tokens,
                                            ));
                                        } else {
                                            tokens.push(Token::Tag(head_tag, sub_tokens));
                                        }
                                    }
                                    _ => panic!("Tag must be balanced"),
                                }
                            } else {
                                panic!("Missing opening tag");
                            }
                        }
                        Tag::If(_, _, _) => {
                            tag_token_stack.push(Token::Tag(tag, Vec::new()));
                        }
                        Tag::EndIf => {
                            if let Some(Token::Tag(head_tag, mut sub_tokens)) =
                                tag_token_stack.pop()
                            {
                                match head_tag {
                                    Tag::If(_, _, _) => {
                                        sub_tokens.push(Token::Text(last_pos, i));
                                        last_pos = i;
                                        if let Some(Token::Tag(
                                            parent_head_tag,
                                            mut parent_sub_tokens,
                                        )) = tag_token_stack.pop()
                                        {
                                            parent_sub_tokens
                                                .push(Token::Tag(head_tag, sub_tokens));
                                            tag_token_stack.push(Token::Tag(
                                                parent_head_tag,
                                                parent_sub_tokens,
                                            ));
                                        } else {
                                            tokens.push(Token::Tag(head_tag, sub_tokens));
                                        }
                                    }
                                    _ => panic!("Tag must be balanced"),
                                }
                            } else {
                                panic!("Missing opening tag");
                            }
                        }
                    }
                    last_pos = i + 2;
                    i += 2;
                } else {
                    panic!("Symbols must be balanced: {{% %}}");
                }
            }
            (b'{', b'$') => {
                if last_pos < i {
                    tokens.push(Token::Text(last_pos, i));
                    last_pos = i;
                }
                head_symbol_stack.push((Symbol::Env, i + 2));
                last_pos += 2;
                i += 2;
            }
            (b'$', b'}') => {
                if let Some((Symbol::Env, start_idx)) = head_symbol_stack.pop() {
                    tokens.push(Token::Env(start_idx, i));
                    last_pos = i + 2;
                } else {
                    panic!("Symbols must be balanced: {{$ $}}");
                }
                i += 2;
            }
            (b'{', b'{') => {
                if last_pos < i {
                    tokens.push(Token::Text(last_pos, i));
                    last_pos = i;
                }
                head_symbol_stack.push((Symbol::Placeholder, i + 2));
                last_pos += 2;
                i += 2;
            }
            (b'}', b'}') => {
                if let Some((Symbol::Placeholder, start_idx)) = head_symbol_stack.pop() {
                    tokens.push(Token::Placeholder(start_idx, i));
                    last_pos = i + 2;
                    i += 2;
                } else {
                    panic!("Symbols must be balanced: {{{{ }}}}");
                }
            }
            _ => i += 1,
        }
    }
    tokens.push(Token::Text(last_pos, bytes.len()));
    tokens
}

#[derive(Debug)]
enum Tag<'a> {
    For(&'a str, &'a str),
    EndFor,
    If(&'a str, &'a str, &'a str),
    EndIf,
}

fn generate_tag(tag_bytes: &[u8]) -> Tag {
    let tag_text = str::from_utf8(tag_bytes)
        .expect("Convert to str fail")
        .trim();
    match tag_text {
        "endfor" => Tag::EndFor,
        "endif" => Tag::EndIf,
        _ => {
            if tag_text.starts_with("for ") {
                let tag_slices: Vec<&str> = tag_text.splitn(4, ' ').collect();
                if tag_slices.len() != 4 || *tag_slices.get(2).unwrap() != "in" {
                    panic!("Illegal expression: for")
                }
                let item_name = tag_slices.get(1).unwrap();
                let collect_name = tag_slices.get(3).unwrap();
                Tag::For(item_name, collect_name)
            } else if tag_text.starts_with("if ") {
                let tag_slices: Vec<&str> = tag_text.splitn(4, ' ').collect();
                if tag_slices.len() != 4 || *tag_slices.get(2).unwrap() != "==" {
                    panic!("Illegal expression: if")
                }
                let exprn_left = tag_slices.get(1).unwrap();
                let exprn_right = tag_slices.get(3).unwrap();
                Tag::If(exprn_left, "==", exprn_right)
            } else {
                panic!("Unsupported tag")
            }
        }
    }
}

fn fill(template_content_bytes: &[u8], tokens: Vec<Token>, data: &Value) -> String {
    let mut filled = String::new();

    let bytes = template_content_bytes;
    for token in tokens {
        match token {
            Token::Tag(tag, sub_tokens) => {
                todo!()
            }
            Token::Text(start, end) => {
                filled.push_str(
                    str::from_utf8(&bytes[start..end]).expect("Convert &[u8] to &str fail"),
                );
            }
            _ => (),
        }
    }

    filled
}
