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
    Raw,
}

#[derive(Debug)]
enum Token<'a> {
    Text(usize, usize),
    Placeholder(usize, usize),
    Env(usize, usize),
    Tag(Tag<'a>, Vec<Token<'a>>),
}

struct GenerateTokensContext<'a> {
    pub tokens: Vec<Token<'a>>,
    pub head_symbol_stack: Vec<(Symbol, usize)>,
    pub tag_token_stack: Vec<Token<'a>>,
    pub now_in_raw_symbol: bool,
    pub last_pos: usize,
}

impl<'a> GenerateTokensContext<'a> {
    fn new() -> Self {
        Self {
            tokens: Vec::new(),
            head_symbol_stack: Vec::with_capacity(1),
            tag_token_stack: Vec::new(),
            now_in_raw_symbol: false,
            last_pos: 0,
        }
    }

    pub fn push_token(&mut self, token: Token<'a>) {
        if self.tag_token_stack.is_empty() {
            self.tokens.push(token);
        } else {
            if let Token::Tag(_, sub_tokens) = self.tag_token_stack.last_mut().unwrap() {
                sub_tokens.push(token);
            } else {
                panic!("An impossible error")
            }
        }
    }
}

fn generate_tokens(template_content_bytes: &[u8]) -> Vec<Token> {
    let mut ctx = GenerateTokensContext::new();

    let bytes = template_content_bytes;
    let mut i = 0;
    let len = bytes.len().saturating_sub(1);
    while i < len {
        // Raw
        if ctx.now_in_raw_symbol {
            if bytes[i] == b'#' && bytes[i + 1] == b'}' {
                if let Some((Symbol::Raw, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    ctx.push_token(Token::Text(start_idx, i));
                    ctx.last_pos = i + 2;
                }
                i += 2;
                ctx.now_in_raw_symbol = false;
                continue;
            } else {
                i += 1;
                continue;
            }
        }

        match (&bytes[i], &bytes[i + 1]) {
            (b'{', b'%') => {
                if ctx.last_pos < i {
                    ctx.push_token(Token::Text(ctx.last_pos, i));
                    ctx.last_pos = i;
                }
                ctx.head_symbol_stack.push((Symbol::Logical, i + 2));
                ctx.last_pos += 2;
                i += 2;
            }
            (b'%', b'}') => {
                if let Some((Symbol::Logical, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    let tag = generate_tag(&bytes[start_idx..i]);
                    match tag {
                        Tag::For(_, _) => {
                            ctx.tag_token_stack.push(Token::Tag(tag, Vec::new()));
                        }
                        Tag::EndFor => {
                            if let Some(Token::Tag(head_tag, sub_tokens)) =
                                ctx.tag_token_stack.pop()
                            {
                                match head_tag {
                                    Tag::For(_, _) => {
                                        ctx.push_token(Token::Tag(head_tag, sub_tokens));
                                    }
                                    _ => panic!("Tag must be balanced"),
                                }
                            } else {
                                panic!("Missing opening tag");
                            }
                        }
                        Tag::If(_, _, _) => {
                            ctx.tag_token_stack.push(Token::Tag(tag, Vec::new()));
                        }
                        Tag::EndIf => {
                            if let Some(Token::Tag(head_tag, sub_tokens)) =
                                ctx.tag_token_stack.pop()
                            {
                                match head_tag {
                                    Tag::If(_, _, _) => {
                                        ctx.push_token(Token::Tag(head_tag, sub_tokens));
                                    }
                                    _ => panic!("Tag must be balanced"),
                                }
                            } else {
                                panic!("Missing opening tag");
                            }
                        }
                    }
                    ctx.last_pos = i + 2;
                }
                i += 2;
            }
            (b'{', b'$') => {
                if ctx.last_pos < i {
                    ctx.push_token(Token::Text(ctx.last_pos, i));
                    ctx.last_pos = i;
                }
                ctx.head_symbol_stack.push((Symbol::Env, i + 2));
                ctx.last_pos += 2;
                i += 2;
            }
            (b'$', b'}') => {
                if let Some((Symbol::Env, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    ctx.push_token(Token::Env(start_idx, i));
                    ctx.last_pos = i + 2;
                }
                i += 2;
            }
            (b'{', b'{') => {
                if ctx.last_pos < i {
                    ctx.push_token(Token::Text(ctx.last_pos, i));
                    ctx.last_pos = i;
                }
                ctx.head_symbol_stack.push((Symbol::Placeholder, i + 2));
                ctx.last_pos += 2;
                i += 2;
            }
            (b'}', b'}') => {
                if let Some((Symbol::Placeholder, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    ctx.push_token(Token::Placeholder(start_idx, i));
                    ctx.last_pos = i + 2;
                }
                i += 2;
            }
            (b'{', b'#') => {
                if ctx.last_pos < i {
                    ctx.push_token(Token::Text(ctx.last_pos, i));
                    ctx.last_pos = i;
                }
                ctx.now_in_raw_symbol = true;
                ctx.head_symbol_stack.push((Symbol::Raw, i + 2));
                ctx.last_pos += 2;
                i += 2;
            }
            _ => i += 1,
        }
    }
    ctx.push_token(Token::Text(ctx.last_pos, bytes.len()));
    ctx.tokens
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
                panic!("Unsupported tag: {}", tag_text)
            }
        }
    }
}

fn fill(template_content_bytes: &[u8], tokens: Vec<Token>, data: &Value) -> String {
    let mut filled = String::new();

    let bytes = template_content_bytes;
    for token in tokens {
        match token {
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
