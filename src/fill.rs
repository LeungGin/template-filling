use std::{cell::RefCell, collections::HashMap, mem, rc::Rc, str};

use chrono::Local;
use serde_json::{json, Value};

pub fn fill_template(template_content: String, data: &Value) -> String {
    // Generate tokens
    let bytes = template_content.as_bytes();
    let tokens = generate_tokens(bytes);
    // debug
    println!("{:?}", tokens);
    // Fill with token
    fill(bytes, &tokens, &mut AutoDataContext::new(data))
}

#[derive(Debug)]
enum Symbol {
    Logical,
    Env,
    Placeholder,
    Raw,
}

#[derive(Debug)]
enum Token {
    Text(TokenContext),
    Placeholder(TokenContext),
    Env(TokenContext),
    Tag(TokenContext, TagExtend),
}

#[derive(Debug)]
struct TokenContext {
    start: usize, // 【T2】tag类token没有记录start和end，默认应该记录head的start和end，然后可以考虑添加属性记录tail的start和end
    end: usize,
    /// Determine by checking the 'tag_token_stack' in GenerateTokensContext, false if empty
    in_tag: bool,
    first_in_row: Option<bool>, // 【T2】TODO 改为每行都记录indent后没必要是Option类型了
    /// Vec<(indent_index_start, indent_index_end)>
    indent: Option<Vec<(usize, usize)>>,
    // TODO 【T0】tag内内容缩进控制（设计3种缩进类型：indent_base = inherit/tag/raw）
}

#[derive(Debug)]
struct TagExtend {
    tag: Tag,
    sub_tokens: Vec<Token>,
}

impl Token {
    pub fn new_text(ctx: &GenerateTokensContext, start: usize, end: usize) -> Token {
        Token::Text(TokenContext {
            start,
            end,
            in_tag: ctx.now_in_tag(),
            first_in_row: None,
            indent: None,
        })
    }

    pub fn new_placeholder(ctx: &GenerateTokensContext, start: usize, end: usize) -> Token {
        Token::Placeholder(TokenContext {
            start,
            end,
            in_tag: ctx.now_in_tag(),
            first_in_row: None,
            indent: None,
        })
    }

    pub fn new_env(ctx: &GenerateTokensContext, start: usize, end: usize) -> Token {
        Token::Env(TokenContext {
            start,
            end,
            in_tag: ctx.now_in_tag(),
            first_in_row: None,
            indent: None,
        })
    }

    pub fn new_tag(ctx: &GenerateTokensContext, tag: Tag) -> Token {
        Token::Tag(
            TokenContext {
                start: 0,
                end: 0,
                in_tag: ctx.now_in_tag(),
                first_in_row: None,
                indent: None,
            },
            TagExtend {
                tag,
                sub_tokens: Vec::new(),
            },
        )
    }
}

struct GenerateTokensContext {
    pub last_pos: usize,
    pub tokens: Vec<Token>,

    // <<< Keep coding, time will reward --- 2025/5/22 1:01 >>>
    pub now_in_raw: bool,
    pub now_has_first_non_blank: bool,
    /// Index start and end of indent text.
    /// There may be multiple separated whitespace characters,
    /// for example (The * symbol stands for whitespace characters),
    /// ```
    /// *******<$ custom_env = 123 $>***
    /// ```
    pub indent_in_row: Vec<(usize, usize)>,

    pub head_symbol_stack: Vec<(Symbol, usize)>,
    pub tag_token_stack: Vec<Token>,
}

impl<'a> GenerateTokensContext {
    fn new() -> Self {
        Self {
            last_pos: 0,
            tokens: Vec::new(),
            now_in_raw: false,
            now_has_first_non_blank: false,
            indent_in_row: Vec::new(),
            head_symbol_stack: Vec::with_capacity(1),
            tag_token_stack: Vec::new(),
        }
    }

    /// When row is break, should run this function to reset status
    pub fn reset_row_status(&mut self) {
        self.now_has_first_non_blank = false;
        self.indent_in_row.clear();
    }

    /// Use to record status about head tag
    pub fn push_tag_token_stack(&mut self, mut token: Token) {
        if let Token::Tag(token_ctx, _) = &mut token {
            if !self.now_has_first_non_blank {
                self.now_has_first_non_blank = true;
                token_ctx.in_tag = false;
                token_ctx.first_in_row = Some(true);
                token_ctx.indent = Some(mem::replace(&mut self.indent_in_row, Vec::new()));
                self.tag_token_stack.push(token);
            }
        }
    }

    pub fn push_token(&mut self, mut token: Token, template_bytes: &[u8]) {
        if self.now_has_first_non_blank {
            self.push_token_0(token);
            return;
        }
        match &mut token {
            Token::Text(token_ctx) => {
                let start = token_ctx.start;
                let end = token_ctx.end;
                let text = bytes_to_str(template_bytes, start, end);
                let non_blank_text = text
                    .find(|c: char| !c.is_whitespace())
                    .map(|pos| &text[pos..]);
                if let Some(non_blank_text) = non_blank_text {
                    self.now_has_first_non_blank = true;
                    let non_blank_len = non_blank_text.len();
                    if non_blank_len < text.len() {
                        self.indent_in_row
                            .push((start, start + (text.len() - non_blank_len)));
                    }
                    token_ctx.first_in_row = Some(true);
                    token_ctx.indent = Some(mem::replace(&mut self.indent_in_row, Vec::new()));
                    token_ctx.start = end - non_blank_len;
                    self.push_token_0(token);
                } else {
                    self.indent_in_row.push((start, end));
                }
            }
            Token::Placeholder(token_ctx) | Token::Env(token_ctx) | Token::Tag(token_ctx, _) => {
                self.now_has_first_non_blank = true;
                token_ctx.first_in_row = Some(true);
                // Tokens with head and tail, using the indent of the head
                if token_ctx.indent.is_none() {
                    token_ctx.indent = Some(mem::replace(&mut self.indent_in_row, Vec::new()));
                }
                self.push_token_0(token);
            }
        }
    }

    fn push_token_0(&mut self, token: Token) {
        if self.tag_token_stack.is_empty() {
            self.tokens.push(token);
        } else {
            if let Token::Tag(_, TagExtend { sub_tokens, .. }) =
                self.tag_token_stack.last_mut().unwrap()
            {
                sub_tokens.push(token);
            } else {
                panic!("An impossible error when push token")
            }
        }
    }

    pub fn now_in_tag(&self) -> bool {
        !self.tag_token_stack.is_empty()
    }
}

fn generate_tokens(template_bytes: &[u8]) -> Vec<Token> {
    let mut ctx = GenerateTokensContext::new();

    let bytes = template_bytes;
    let mut i = 0;
    let len = bytes.len().saturating_sub(1);
    while i < len {
        // Raw
        if ctx.now_in_raw {
            if bytes[i] == b'#' && bytes[i + 1] == b'}' {
                if let Some((Symbol::Raw, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    ctx.push_token(Token::new_text(&ctx, start_idx, i), template_bytes);
                    ctx.last_pos = i + 2;
                }
                i += 2;
                ctx.now_in_raw = false;
                continue;
            } else {
                i += 1;
                continue;
            }
        }

        match (&bytes[i], &bytes[i + 1]) {
            (b'{', b'%') => {
                if ctx.last_pos < i {
                    ctx.push_token(Token::new_text(&ctx, ctx.last_pos, i), template_bytes);
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
                            ctx.push_tag_token_stack(Token::new_tag(&ctx, tag));
                        }
                        Tag::EndFor => {
                            if let Some(tag_token) = ctx.tag_token_stack.pop() {
                                if let Token::Tag(_, TagExtend { tag, .. }) = &tag_token {
                                    match tag {
                                        Tag::For(_, _) => {
                                            ctx.push_token(tag_token, template_bytes);
                                        }
                                        _ => panic!("Tag must be balanced"),
                                    }
                                } else {
                                    panic!("Missing head tag");
                                }
                            }
                        }
                        Tag::If(_, _, _) => {
                            ctx.push_tag_token_stack(Token::new_tag(&ctx, tag));
                        }
                        Tag::EndIf => {
                            let token = ctx.tag_token_stack.pop();
                            if let Some(Token::Tag(_, ref ext)) = token {
                                match ext.tag {
                                    Tag::If(_, _, _) => {
                                        ctx.push_token(token.unwrap(), template_bytes);
                                    }
                                    _ => panic!("Tag must be balanced"),
                                }
                            } else {
                                panic!("Missing head tag");
                            }
                        }
                    }
                    ctx.last_pos = i + 2;
                }
                i += 2;
            }
            (b'{', b'$') => {
                if ctx.last_pos < i {
                    ctx.push_token(Token::new_text(&ctx, ctx.last_pos, i), template_bytes);
                    ctx.last_pos = i;
                }
                ctx.head_symbol_stack.push((Symbol::Env, i + 2));
                ctx.last_pos += 2;
                i += 2;
            }
            (b'$', b'}') => {
                if let Some((Symbol::Env, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    if !bytes[start_idx..i].contains(&b'=') {
                        panic!("Env symbol missing '=', it should be define like '{{$ key = value $}}'")
                    }
                    ctx.push_token(Token::new_env(&ctx, start_idx, i), template_bytes);
                    ctx.last_pos = i + 2;
                }
                i += 2;
            }
            (b'{', b'{') => {
                if ctx.last_pos < i {
                    ctx.push_token(Token::new_text(&ctx, ctx.last_pos, i), template_bytes);
                    ctx.last_pos = i;
                }
                ctx.head_symbol_stack.push((Symbol::Placeholder, i + 2));
                ctx.last_pos += 2;
                i += 2;
            }
            (b'}', b'}') => {
                if let Some((Symbol::Placeholder, _)) = ctx.head_symbol_stack.last() {
                    let (_, start_idx) = ctx.head_symbol_stack.pop().unwrap();
                    ctx.push_token(Token::new_placeholder(&ctx, start_idx, i), template_bytes);
                    ctx.last_pos = i + 2;
                }
                i += 2;
            }
            (b'{', b'#') => {
                if ctx.last_pos < i {
                    ctx.push_token(Token::new_text(&ctx, ctx.last_pos, i), template_bytes);
                    ctx.last_pos = i;
                }
                ctx.now_in_raw = true;
                ctx.head_symbol_stack.push((Symbol::Raw, i + 2));
                ctx.last_pos += 2;
                i += 2;
            }
            (b'\r', b'\n') => {
                ctx.push_token(Token::new_text(&ctx, ctx.last_pos, i + 2), template_bytes);
                ctx.last_pos = i + 2;
                ctx.reset_row_status();
                i += 2;
            }
            (b'\n', _) => {
                ctx.push_token(Token::new_text(&ctx, ctx.last_pos, i + 1), template_bytes);
                ctx.last_pos = i + 1;
                ctx.reset_row_status();
                i += 1;
            }
            _ => i += 1,
        }
    }
    ctx.push_token(
        Token::new_text(&ctx, ctx.last_pos, bytes.len()),
        template_bytes,
    );
    ctx.tokens
}

#[derive(Debug)]
enum Tag {
    /// for [item] in [array]
    For(String, String),
    EndFor,
    /// if [left] [operator] [right]
    If(String, String, String),
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
                let item_name = tag_slices.get(1).unwrap().to_string();
                let collect_name = tag_slices.get(3).unwrap().to_string();
                Tag::For(item_name, collect_name)
            } else if tag_text.starts_with("if ") {
                let tag_slices: Vec<&str> = tag_text.splitn(4, ' ').collect();
                if tag_slices.len() != 4 || *tag_slices.get(2).unwrap() != "==" {
                    panic!("Illegal expression: if")
                }
                let exprn_left = tag_slices.get(1).unwrap().to_string();
                let exprn_right = tag_slices.get(3).unwrap().to_string();
                Tag::If(exprn_left, "==".to_string(), exprn_right)
            } else {
                panic!("Unsupported tag: {}", tag_text)
            }
        }
    }
}

struct AutoDataContext<'a> {
    scope_stack: Rc<RefCell<Vec<Value>>>,
    sys: HashMap<&'a str, String>,
    data: &'a Value,
}

impl<'a> AutoDataContext<'a> {
    pub fn new(data: &'a Value) -> Self {
        let mut s = Self {
            sys: HashMap::new(),
            scope_stack: Rc::new(RefCell::new(Vec::new())),
            data,
        };
        // setting system env value
        s.set_sys("@now", Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        // Add root scope
        s.push_scope();
        s
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        // 1st, scope (step-by-step loop)
        for scope in Rc::clone(&self.scope_stack).borrow().iter().rev() {
            if let Some(v) = self.get_string_by_step_in_key(scope, key) {
                return Some(v.to_string());
            }
        }
        // 2nd, system env
        if let Some(v) = self.sys.get(key) {
            return Some(v.to_string());
        }
        // 3th, custom global data(step-by-step loop)
        self.get_string_by_step_in_key(self.data, key)
    }

    pub fn get_array(&self, key: &str) -> Option<Vec<Value>> {
        // 1st, scope (step-by-step loop)
        for scope in Rc::clone(&self.scope_stack).borrow().iter().rev() {
            if let Some(val) = self.get_by_step_in_key(scope, key) {
                if val.is_array() {
                    return val.as_array().cloned();
                }
            }
        }
        // 2nd, custom global data(step-by-step loop)
        if let Some(val) = self.get_by_step_in_key(self.data, key) {
            if val.is_array() {
                return val.as_array().cloned();
            }
        }
        None
    }

    fn get_string_by_step_in_key(&self, data: &Value, key: &str) -> Option<String> {
        if let Some(val) = self.get_by_step_in_key(data, key) {
            return Some(self.to_pure_string(&val));
        }
        None
    }

    fn get_by_step_in_key<'b>(&self, data: &'b Value, key: &str) -> Option<&'b Value> {
        if !key.contains(".") {
            return data.get(key).map_or(None, |v| Some(v));
        }

        let mut target = data;
        let keys: Vec<&str> = key.split(".").filter(|&x| !x.is_empty()).collect();
        for k in keys {
            if !target.is_object() {
                return None;
            }
            let v = target.get(k);
            if v.is_none() {
                return None;
            }
            target = v.unwrap();
        }
        Some(target)
    }

    fn to_pure_string(&self, v: &Value) -> String {
        if v.is_string() {
            v.as_str().unwrap().to_owned()
        } else {
            v.to_string()
        }
    }

    pub fn set_scope_with_string(&self, key: &'a str, val: String) {
        if let Some(scope) = Rc::clone(&self.scope_stack).borrow_mut().last_mut() {
            scope[key] = Value::String(val);
            return;
        }
        panic!("No data scope be found, need to add scope first")
    }

    pub fn set_scope_with_value(&self, key: &'a str, val: Value) {
        if let Some(scope) = Rc::clone(&self.scope_stack).borrow_mut().last_mut() {
            scope[key] = val;
            return;
        }
        panic!("No data scope be found, need to add scope first")
    }

    pub fn set_sys(&mut self, key: &'a str, val: String) {
        self.sys.insert(key, val);
    }

    pub fn push_scope(&self) {
        Rc::clone(&self.scope_stack).borrow_mut().push(json!({}));
    }

    pub fn pop_scope(&mut self) {
        Rc::clone(&self.scope_stack).borrow_mut().pop();
    }
}

fn fill(template_bytes: &[u8], tokens: &Vec<Token>, data_ctx: &mut AutoDataContext) -> String {
    let mut filled = String::new();

    for token in tokens {
        match token {
            Token::Tag(_, ext) => match &ext.tag {
                Tag::For(item_key, array_key) => {
                    if let Some(array) = data_ctx.get_array(&array_key) {
                        data_ctx.push_scope();
                        data_ctx.set_scope_with_string("@max", (array.len() - 1).to_string());
                        for i in 0..array.len() {
                            let item = array.get(i).unwrap();
                            if item.is_object() {
                                data_ctx.push_scope();
                                data_ctx.set_scope_with_string("@index", i.to_string());
                                data_ctx.set_scope_with_value(&item_key, item.clone());
                                let replaced = fill(template_bytes, &ext.sub_tokens, data_ctx);
                                filled.push_str(&replaced);
                                data_ctx.pop_scope();
                            }
                        }
                        data_ctx.pop_scope();
                    }
                }
                Tag::If(left, operator, right) => match operator.as_str() {
                    "==" => {
                        let left = get_variable_in_tag(&data_ctx, &left);
                        let right = get_variable_in_tag(&data_ctx, &right);
                        if left.is_some() && right.is_some() && left.unwrap() == right.unwrap() {
                            let replaced = fill(template_bytes, &ext.sub_tokens, data_ctx);
                            filled.push_str(&replaced);
                        }
                    }
                    _ => panic!("Unsupported if's operator: {}", operator),
                },
                _ => panic!("An impossible error when parse tag token"),
            },
            Token::Env(TokenContext { start, end, .. }) => {
                let (k, v) = bytes_to_str(template_bytes, *start, *end)
                    .split_once("=")
                    .unwrap();
                data_ctx.set_scope_with_string(k, v.to_owned());
            }
            Token::Placeholder(TokenContext { start, end, .. }) => {
                let placeholder = bytes_to_str(template_bytes, *start, *end);
                let replaced = match data_ctx.get_string(placeholder) {
                    Some(v) => v,
                    None => format!("{{{{{}}}}}", placeholder),
                };
                filled.push_str(&replaced);
            }
            Token::Text(token_ctx) => {
                filled.push_str(bytes_to_str(template_bytes, token_ctx.start, token_ctx.end));
            }
        }
    }

    filled
}

fn bytes_to_str(bytes: &[u8], start: usize, end: usize) -> &str {
    str::from_utf8(&bytes[start..end]).expect("Convert &[u8] to &str fail")
}

fn get_variable_in_tag(data_ctx: &AutoDataContext, variable: &str) -> Option<String> {
    if variable.len() > 1 {
        if variable.starts_with("@") {
            return data_ctx.get_string(&variable);
        } else if variable.starts_with("$") {
            return data_ctx.get_string(&variable[1..variable.len()]);
        }
    }
    Some(variable.to_string())
}
